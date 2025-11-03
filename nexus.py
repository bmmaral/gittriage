#!/usr/bin/env python3
"""nexus - Dead simple project memory tool"""

import os
import json
import re
import stat
from datetime import datetime
from pathlib import Path
from dotenv import load_dotenv

import click
import git
import httpx
import yaml


class Nexus:
    def __init__(self) -> None:
        load_dotenv()
        self.repo = git.Repo('.')
        self.root = Path('.')
        # Configurable conversations directory via env (.env supported by server)
        self.conv_dir = Path(os.getenv('NEXUS_CONVERSATIONS_DIR', 'conversations')).expanduser()
        self.openrouter_api_key = os.getenv('OPENROUTER_API_KEY')

    def init(self) -> None:
        """Initialize nexus in current repo"""
        # Create folders
        (self.conv_dir).mkdir(exist_ok=True)
        (self.root / 'reports').mkdir(exist_ok=True)
        (self.root / '.nexus').mkdir(exist_ok=True)

        # Create default config if missing
        config_path = self.root / '.nexus' / 'config.yml'
        if not config_path.exists():
            config = {
                'reminder_days': 5,
                'ai_platforms': ['chatgpt', 'claude', 'gemini'],
                'auto_commit': True,
                'analyze_on_push': True,
                'ignore_patterns': ['node_modules', '.env', 'build/'],
            }
            # Write UTF-8 to avoid Windows 'charmap' errors
            with open(config_path, 'w', encoding='utf-8') as f:
                yaml.safe_dump(config, f, sort_keys=False)

        # Add pre-commit hook (idempotent)
        hook_path = self.root / '.git' / 'hooks' / 'pre-commit'
        hook_content = """#!/bin/bash
# Auto-summarize PRD changes
if git diff --cached --name-only | grep -q "PRD.md"; then
    if command -v nexus >/dev/null 2>&1; then
        nexus prd-summary >> .git/COMMIT_EDITMSG
    else
        python3 nexus.py prd-summary >> .git/COMMIT_EDITMSG 2>/dev/null || true
    fi
fi
"""
        try:
            with open(hook_path, 'w') as f:
                f.write(hook_content)
            os.chmod(hook_path, os.stat(hook_path).st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
        except FileNotFoundError:
            # Not a git repo or hooks dir missing; skip silently
            pass

        click.echo("✅ Nexus initialized!")

    def import_conversation(self, file_path: str, platform: str | None = None) -> None:
        """Import AI conversation from a JSON export file"""
        file_path = os.path.expanduser(file_path)
        if not os.path.exists(file_path):
            raise click.ClickException(f"File not found: {file_path}")

        # Auto-detect platform from filename
        if not platform:
            lower = file_path.lower()
            if 'chatgpt' in lower or 'openai' in lower:
                platform = 'chatgpt'
            elif 'claude' in lower or 'anthropic' in lower:
                platform = 'claude'
            elif 'gemini' in lower or 'bard' in lower:
                platform = 'gemini'
            else:
                platform = 'unknown'

        # Copy to conversations folder with date prefix
        date_str = datetime.now().strftime('%Y-%m-%d')
        dest_file = str(self.conv_dir / f"{date_str}-{platform}.json")

        # Always read conversations as UTF-8; fall back gracefully
        with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
            try:
                data = json.load(f)
            except json.JSONDecodeError as e:
                raise click.ClickException(f"Invalid JSON: {e}")

        # Extract naive key decisions (simple heuristics)
        decisions: list[str] = []
        messages = data.get('messages') or data.get('items') or []
        for msg in messages:
            content = (
                msg.get('content')
                or (msg.get('text') if isinstance(msg.get('text'), str) else None)
                or ''
            )
            content_str = str(content)
            lowered = content_str.lower()
            if any(kw in lowered for kw in ['decided', 'will use', "let's go with", 'we will', 'choose']):
                decisions.append(content_str[:160])

        # Save file
        # Save as UTF-8 to preserve emojis and non-ascii
        with open(dest_file, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=2, ensure_ascii=False)

        # Auto-commit if configured
        auto_commit = True
        cfg_path = Path('.nexus/config.yml')
        if cfg_path.exists():
            try:
                cfg = yaml.safe_load(cfg_path.read_text()) or {}
                auto_commit = bool(cfg.get('auto_commit', True))
            except Exception:
                auto_commit = True
        if auto_commit:
            self.repo.index.add([dest_file])
            self.repo.index.commit(f"conv: Import {platform} ({len(decisions)} decisions)")

        click.echo(f"✅ Imported {platform} conversation")
        click.echo(f"📌 Found {len(decisions)} decisions")

        # Update timeline
        self.update_timeline()

    def _get_git_diff(self, file_path, max_lines=50) -> str:
        """Return git diff for a file, handling various scenarios."""
        try:
            # Check if the file is tracked
            self.repo.git.ls_files(file_path, error_unmatch=True)

            # Staged changes (for pre-commit analysis)
            diff_output = self.repo.git.diff('--cached', file_path)

            # Unstaged changes if no staged changes found
            if not diff_output:
                diff_output = self.repo.git.diff(file_path)

            # Diff from last commit if no working directory changes
            if not diff_output:
                diff_output = self.repo.git.diff('HEAD~1', 'HEAD', '--', file_path)

            return '\n'.join(diff_output.splitlines()[:max_lines])

        except git.exc.GitCommandError:
            # File is not tracked, return its content as "new file" diff
            try:
                content = Path(file_path).read_text(encoding='utf-8', errors='ignore')
                # Mimic diff format for new files
                return f"--- /dev/null\n+++ b/{file_path}\n" + '\n'.join([f"+{line}" for line in content.splitlines()[:max_lines]])
            except FileNotFoundError:
                return "" # Should not happen if called on existing files
        except Exception:
            return "" # Graceful fallback

    def _get_ai_summary(self, file_path: str, diff: str) -> str:
        """Get AI-powered summary for a diff."""
        if not self.openrouter_api_key:
            return "AI summary disabled. Set OPENROUTER_API_KEY in .env file."

        # Simplified prompt construction
        if 'prd' in file_path.lower():
            prompt = f"Analyze the following PRD diff for '{file_path}'. Summarize the key changes and their potential impact on the project scope or technical requirements. Focus on strategic implications."
        elif any(file_path.endswith(ext) for ext in ['.js', '.py', '.ts']):
            prompt = f"Analyze the following code diff for '{file_path}'. Summarize the functional changes, potential bugs, or improvements. Highlight any new dependencies or architectural modifications."
        else:
            prompt = f"Analyze the following diff for '{file_path}'. Provide a concise summary of the changes."

        full_prompt = f"{prompt}\n\n---\n\n{diff}"

        try:
            response = httpx.post(
                "https://openrouter.ai/api/v1/chat/completions",
                headers={
                    "Authorization": f"Bearer {self.openrouter_api_key}",
                    "Content-Type": "application/json"
                },
                json={
                    "model": "openrouter/auto",
                    "messages": [{"role": "user", "content": full_prompt}]
                },
                timeout=20.0
            )
            response.raise_for_status()
            data = response.json()
            return data['choices'][0]['message']['content'].strip()
        except httpx.HTTPStatusError as e:
            return f"Error from AI API: {e.response.status_code}"
        except Exception:
            return "Failed to get AI summary."

    def analyze(self, max_files=10) -> None:
        """Analyze code-PRD drift and write reports/drift.md"""
        (Path('reports')).mkdir(exist_ok=True)
        report_lines: list[str] = []
        report_lines.append("# Code-PRD Drift Report\n")
        report_lines.append(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M')}\n\n")

        # Prioritize PRD, then source files, then config files
        prds = sorted(Path('.').glob('prd*.md'), key=lambda p: p.stat().st_mtime, reverse=True)
        source_files = sorted(Path('.').rglob('*.js'), key=lambda p: p.stat().st_mtime, reverse=True)
        config_files = sorted(Path('.').rglob('*.yml'), key=lambda p: p.stat().st_mtime, reverse=True)

        # Combine and limit files to analyze
        files_to_analyze = [str(p) for p in prds + source_files + config_files][:max_files]

        analysis_items = []
        for file in files_to_analyze:
            diff = self._get_git_diff(file)
            if diff:
                analysis_items.append({'file': file, 'diff': diff})

        # Simple heuristic: if PRD changed, it's high priority
        priority_items = [item for item in analysis_items if 'prd' in item['file'].lower()]
        other_items = [item for item in analysis_items if 'prd' not in item['file'].lower()]

        report_lines.append("## 🤖 AI-Generated Analysis\n\n")

        for item in priority_items + other_items:
            file, diff = item['file'], item['diff']
            summary = self._get_ai_summary(file, diff)

            report_lines.append(f"### `{file}`\n")
            report_lines.append(f"**💡 AI Summary:** {summary}\n")
            report_lines.append("```diff\n")
            report_lines.append(diff + "\n")
            report_lines.append("```\n\n")

        report_text = ''.join(report_lines)
        Path('reports/drift.md').write_text(report_text, encoding='utf-8')

        issue_count = len(analysis_items)
        click.echo(f"🔍 Analysis complete: {issue_count} files analyzed")

        if issue_count > 0:
            try:
                self.repo.index.add(['reports/drift.md'])
                self.repo.index.commit(f"analyze: AI summary for {issue_count} changed files")
            except Exception:
                pass

    def status(self) -> None:
        """Show project status"""
        last_commit = self.repo.head.commit
        days_ago = (datetime.now() - datetime.fromtimestamp(last_commit.committed_date)).days
        conv_count = len(list(self.conv_dir.glob('*.json'))) if self.conv_dir.exists() else 0
        drift_exists = Path('reports/drift.md').exists()

        click.echo(
            f"""
📊 Project Status
─────────────────
Last Activity: {days_ago} days ago
Last Commit: {last_commit.message.split(chr(10))[0]}
Conversations: {conv_count}
Drift Report: {'✅ Available' if drift_exists else '❌ Not generated'}

{'⚠️  Project inactive for ' + str(days_ago) + ' days!' if days_ago >= 5 else '✅ Project is active'}
"""
        )

    def update_timeline(self) -> None:
        """Generate conversation timeline at conversations/index.md"""
        timeline: list[str] = ["# Conversation Timeline\n\n"]
        events: list[dict] = []

        if self.conv_dir.exists():
            for conv_file in self.conv_dir.glob('*.json'):
                parts = conv_file.stem.split('-')
                date = '-'.join(parts[0:3]) if len(parts) >= 3 else datetime.now().strftime('%Y-%m-%d')
                platform = parts[-1] if parts else 'unknown'
                events.append({'date': date, 'type': 'conversation', 'platform': platform, 'file': str(conv_file)})

        for commit in list(self.repo.iter_commits('HEAD', max_count=10)):
            events.append({
                'date': datetime.fromtimestamp(commit.committed_date).strftime('%Y-%m-%d'),
                'type': 'commit',
                'message': commit.message.split('\n')[0],
                'sha': commit.hexsha[:7],
            })

        events.sort(key=lambda x: x['date'], reverse=True)

        current_date: str | None = None
        for event in events:
            if event['date'] != current_date:
                timeline.append(f"\n### {event['date']}\n\n")
                current_date = event['date']
            if event['type'] == 'conversation':
                timeline.append(f"- 💬 **{event['platform']}** conversation imported\n")
            elif event['type'] == 'commit':
                timeline.append(f"- 📝 [{event['sha']}] {event['message']}\n")

        (self.conv_dir / 'index.md').write_text(''.join(timeline), encoding='utf-8')

    def prd_summary(self) -> str:
        """Generate PRD change summary for commit message"""
        try:
            diff = self.repo.index.diff('HEAD', paths=['PRD.md'])
        except Exception:
            return ""

        if not diff:
            return ""

        # Very naive summary
        changed_sections: list[str] = []
        for d in diff:
            if getattr(d, 'a_path', None) == 'PRD.md' or getattr(d, 'b_path', None) == 'PRD.md':
                changed_sections.append('PRD updated')

        return f"docs: {', '.join(changed_sections)}" if changed_sections else ''

    def check_inactive(self) -> dict:
        """Check inactivity and print JSON for GitHub Action"""
        last_commit = self.repo.head.commit
        days_inactive = (datetime.now() - datetime.fromtimestamp(last_commit.committed_date)).days

        if days_inactive >= 5:
            next_steps = "No next steps defined"
            if Path('PRD.md').exists():
                content = Path('PRD.md').read_text(encoding='utf-8', errors='ignore')
                match = re.search(r'## Next Steps(.*?)##', content, re.DOTALL)
                if match:
                    next_steps = match.group(1).strip()[:500]
            result = {
                'inactive': True,
                'days': days_inactive,
                'next_steps': next_steps,
                'last_commit': str(last_commit.message),
            }
            print(json.dumps(result))
            return result

        return {'inactive': False}


@click.group()
def cli() -> None:
    """Nexus - Project memory tool"""
    pass


@cli.command()
def init() -> None:
    """Initialize nexus in current repo"""
    Nexus().init()


@cli.command(name='import-conv')
@click.argument('file')
@click.option('--platform', help='Platform (chatgpt/claude/gemini)')
def import_conv_cmd(file: str, platform: str | None) -> None:
    """Import conversation file (legacy alias)"""
    Nexus().import_conversation(file, platform)


@cli.group(name='add')
def add_group() -> None:
    """Add resources to the project (conversations, modules)"""
    pass


@add_group.command(name='conversation')
@click.argument('file')
@click.option('--platform', help='Platform (chatgpt/claude/gemini)')
def add_conversation_cmd(file: str, platform: str | None) -> None:
    """Add (import) an AI conversation export file"""
    Nexus().import_conversation(file, platform)


@cli.command()
def analyze() -> None:
    """Analyze code-PRD drift"""
    Nexus().analyze()


@cli.command()
def status() -> None:
    """Show project status"""
    Nexus().status()


@cli.command()
def timeline() -> None:
    """Update conversation timeline"""
    Nexus().update_timeline()
    click.echo("✅ Timeline updated: conversations/index.md")


@cli.command()
def check() -> None:
    """Check for inactive projects (for GitHub Action)"""
    Nexus().check_inactive()


@cli.command(name='remind')
def remind_cmd() -> None:
    """Check inactivity and print a human-readable reminder summary"""
    result = Nexus().check_inactive()
    if result.get('inactive'):
        days = result.get('days')
        last = (result.get('last_commit') or '').split('\n')[0]
        click.echo(
            f"⏰ Project inactive for {days} days. Last commit: {last}"
        )
        click.echo("Tip: Define a '## Next Steps' section in PRD.md for better reminders.")
    else:
        click.echo("✅ Project is active. No reminder needed.")


@cli.command(name='prd-summary')
def prd_summary_cmd() -> None:
    """Print PRD change summary for commit message"""
    summary = Nexus().prd_summary()
    if summary:
        click.echo(summary)


if __name__ == '__main__':
    cli()
