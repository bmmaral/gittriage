#!/usr/bin/env python3
"""nexus - Dead simple project memory tool"""

import os
import json
import re
import stat
from datetime import datetime
from pathlib import Path

import click
import git
import yaml


class Nexus:
    def __init__(self) -> None:
        self.repo = git.Repo('.')
        self.root = Path('.')

    def init(self) -> None:
        """Initialize nexus in current repo"""
        # Create folders
        (self.root / 'conversations').mkdir(exist_ok=True)
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
            with open(config_path, 'w') as f:
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
        dest_file = f"conversations/{date_str}-{platform}.json"

        with open(file_path, 'r') as f:
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
        with open(dest_file, 'w') as f:
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

    def analyze(self) -> None:
        """Analyze code-PRD drift and write reports/drift.md"""
        (Path('reports')).mkdir(exist_ok=True)
        report_lines: list[str] = []
        report_lines.append("# Code-PRD Drift Report\n")
        report_lines.append(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M')}\n\n")

        prd_content = Path('PRD.md').read_text() if Path('PRD.md').exists() else ''

        # Endpoints listed in PRD
        prd_endpoints = set(re.findall(r'(?:GET|POST|PUT|DELETE)\s+(/api/\S+)', prd_content, flags=re.IGNORECASE))
        prd_endpoints |= set(re.findall(r'(/api/\S+)', prd_content))

        # Actual endpoints in code (very simple Express-like scan)
        code_endpoints: list[tuple[str, str, str]] = []  # (method, endpoint, file)
        for file in Path('.').rglob('*.js'):
            try:
                content = file.read_text(errors='ignore')
            except Exception:
                continue
            for match in re.findall(r'app\.(get|post|put|delete)\([\'\"](\S+)[\'\"]', content, flags=re.IGNORECASE):
                method, endpoint = match
                code_endpoints.append((method.upper(), endpoint, str(file)))

        undocumented = [(m, e, f) for (m, e, f) in code_endpoints if e not in prd_endpoints]

        if undocumented:
            report_lines.append("## 🔴 Undocumented Endpoints\n\n")
            for method, endpoint, file in undocumented:
                report_lines.append(f"- `{method} {endpoint}` in `{file}`\n")

        # Recent AI commits
        recent_commits = list(self.repo.iter_commits('HEAD', max_count=20))
        ai_commits = [c for c in recent_commits if any(ai in c.message.lower() for ai in ['cursor', 'copilot', 'ai'])]
        if ai_commits:
            report_lines.append("\n## 🤖 Recent AI Changes\n\n")
            for commit in ai_commits[:5]:
                report_lines.append(f"- {commit.hexsha[:7]}: {commit.message.split('\n')[0]}\n")

        report_text = ''.join(report_lines)
        Path('reports/drift.md').write_text(report_text)

        issue_count = len(undocumented) + len(ai_commits)
        click.echo(f"🔍 Analysis complete: {issue_count} issues found")

        if issue_count > 0:
            # Best-effort commit
            try:
                self.repo.index.add(['reports/drift.md'])
                self.repo.index.commit(f"analyze: {issue_count} drift issues detected")
            except Exception:
                pass

    def status(self) -> None:
        """Show project status"""
        last_commit = self.repo.head.commit
        days_ago = (datetime.now() - datetime.fromtimestamp(last_commit.committed_date)).days
        conv_count = len(list(Path('conversations').glob('*.json'))) if Path('conversations').exists() else 0
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

        if Path('conversations').exists():
            for conv_file in Path('conversations').glob('*.json'):
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

        Path('conversations/index.md').write_text(''.join(timeline))

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
                content = Path('PRD.md').read_text()
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
    """Import conversation file"""
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


@cli.command(name='prd-summary')
def prd_summary_cmd() -> None:
    """Print PRD change summary for commit message"""
    summary = Nexus().prd_summary()
    if summary:
        click.echo(summary)


if __name__ == '__main__':
    cli()
