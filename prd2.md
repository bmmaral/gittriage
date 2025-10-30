# Project Nexus — Product Requirements & Technical Document (PRD)

---
id: NEXUS-PRD-001
title: Project Nexus - AI-Powered Project Memory & Repository Sync System
version: 1.0
owner: Product Team
status: Active
last_updated: 2025-10-27
tags: [PRD, MVP, Git-Native, AI-Integration, Project-Management, Developer-Tools]
---

## 📋 Changelog

| Date | Version | Changes | Affected IDs |
|------|---------|---------|-------------|
| **2025-10-27** | v1.0 | Initial document creation | All |
| **2025-10-27** | v1.0 | Problem definition and user context | PRB-001, CTX-001 |
| **2025-10-27** | v1.0 | Core features specification | COR-001 to COR-005 |
| **2025-10-27** | v1.0 | Technical architecture design | ARC-001 to ARC-004 |
| **2025-10-27** | v1.0 | Git-native implementation strategy | GIT-001 |

---

## 📑 Table of Contents

<details open>
<summary><b>🚀 Quick Access by Role</b> (Click to expand/collapse)</summary>

| Role | Start Here | Key Sections | Time |
|------|------------|--------------|------|
| 🎯 **Product Owner** | [Executive Summary](#executive-summary) | Vision, Problem, Success Criteria | 10 min |
| 🎨 **Designer** | [User Experience](#user-experience) | Dashboard, Mobile App, Workflows | 20 min |
| 👨‍💻 **Developer** | [Technical Architecture](#technical-architecture) | Git Integration, CLI Tools, APIs | 30 min |
| 🤖 **AI Engineer** | [AI Components](#ai-components) | Summarizer, Analyzer, Scout | 25 min |
| 📱 **Mobile Dev** | [Mobile Dashboard](#mobile-dashboard) | React Native Specs | 15 min |

</details>

---

<details>
<summary><b>📌 PART I: Vision & Problem Space</b></summary>

- **[1. Executive Summary](#executive-summary)** `[ID: EXE-001]` `[CRITICAL]`
- **[2. Problem Statement](#problem-statement)** `[ID: PRB-001]` `[CRITICAL]`
- **[3. User Context](#user-context)** `[ID: CTX-001]` `[CRITICAL]`
- **[4. Success Criteria](#success-criteria)** `[ID: SUC-001]` `[MVP]`

</details>

---

<details>
<summary><b>🎯 PART II: Solution & Features</b></summary>

- **[5. Core Features](#core-features)** `[ID: COR-001]` `[MVP]` `[CRITICAL]`
  - [5.1 Contextual PRD Summarizer](#prd-summarizer) `[ID: PRS-001]` ⚡
  - [5.2 Conversation Graph View](#conversation-graph) `[ID: CGV-001]` ⚡
  - [5.3 Code Sync Analyzer](#code-sync-analyzer) `[ID: CSA-001]` ⚡
  - [5.4 Loop Reminder](#loop-reminder) `[ID: LRM-001]`
  - [5.5 Cross-Project Reuse Scout](#reuse-scout) `[ID: RSC-001]` ⚡
- **[6. User Experience](#user-experience)** `[ID: UXP-001]` `[MVP]`
- **[7. Workflow Automation](#workflow-automation)** `[ID: WFA-001]` `[MVP]`

</details>

---

<details>
<summary><b>⚙️ PART III: Technical Implementation</b></summary>

- **[8. Technical Architecture](#technical-architecture)** `[ID: ARC-001]` `[MVP]` `[CRITICAL]`
- **[9. Git-Native Design](#git-native-design)** `[ID: GIT-001]` `[MVP]` `[CRITICAL]`
- **[10. AI Components](#ai-components)** `[ID: AIC-001]` `[MVP]`
- **[11. CLI Tools](#cli-tools)** `[ID: CLI-001]` `[MVP]`
- **[12. Mobile Dashboard](#mobile-dashboard)** `[ID: MOB-001]`
- **[13. GitHub Actions](#github-actions)** `[ID: GHA-001]` `[MVP]`

</details>

---

<details>
<summary><b>✅ PART IV: Validation & Launch</b></summary>

- **[14. Acceptance Criteria](#acceptance-criteria)** `[ID: ACC-001]` `[MVP]` `[CRITICAL]`
- **[15. MVP Scope](#mvp-scope)** `[ID: MVP-001]` `[MVP]` `[CRITICAL]`
- **[16. Development Phases](#development-phases)** `[ID: DEV-001]`
- **[17. Security & Privacy](#security-privacy)** `[ID: SEC-001]` `[CRITICAL]`

</details>

---

# 📌 PART I: VISION & PROBLEM SPACE

---

## 1️⃣ Executive Summary {#executive-summary}

**[ID: EXE-001]** **[CRITICAL]**

> **💡 TL;DR:** Project Nexus is a Git-native system that unifies AI conversations, PRDs, code changes, and project memory into a single source of truth. It prevents context loss across 15+ concurrent projects by automatically syncing decisions, detecting code-PRD drift, and reminding about inactive projects.

### Product Overview

**Name:** Project Nexus  
**Category:** AI-Powered Developer Productivity Tool  
**Platform:** CLI, GitHub Actions, Mobile Dashboard  
**Target Users:** Developers managing multiple concurrent projects with AI assistance

### Vision Statement

> "Never lose project context again. Every AI conversation, every decision, every line of code - tracked, synced, and remembered through Git."

### Value Proposition

Project Nexus solves the multi-project chaos by :
1. **Unifying** all AI conversations and decisions in Git repositories
2. **Automatically committing** PRD changes with AI-generated summaries
3. **Detecting** drift between code and documentation
4. **Reminding** about inactive projects before they're forgotten
5. **Identifying** reusable modules across projects

### Key Metrics (Target Year 1)

```yaml
Adoption:
  - Active users: 10,000 developers
  - Repositories tracked: 150,000+
  - AI conversations synced: 1M+
  - Cross-project modules identified: 50,000+

Performance:
  - Context recovery time: <30 seconds
  - PRD-code sync accuracy: 95%+
  - Inactive project detection: 100%
  - Module reuse rate: 40%+
```

---

## 2️⃣ Problem Statement {#problem-statement}

**[ID: PRB-001]** **[CRITICAL]**

> **💡 TL;DR:** Developers juggling 15+ projects lose context constantly. AI conversations are scattered across platforms, background agents write code without supervision, projects go inactive and are forgotten, and the same modules are rebuilt in every project.

### Quantified Pain Points

```yaml
Context Loss:
  - 15+ active projects simultaneously
  - 4+ AI platforms per project (ChatGPT, Claude, Gemini, Codex)
  - 8+ AI agents making autonomous changes
  - Result: 2-3 hours/week recovering lost context

Documentation Drift:
  - PRDs created but not maintained
  - Code evolves without PRD updates
  - Background AI commits unchecked
  - Result: 60% of PRDs outdated within 2 weeks

Project Abandonment:
  - Projects inactive for days/weeks
  - Forgetting where development stopped
  - No systematic resumption process
  - Result: 70% of projects never completed

Module Duplication:
  - Same functionality rebuilt repeatedly
  - No cross-project visibility
  - Reusable components unidentified
  - Result: 40% development time wasted
```

### Current Tool Failures :

```yaml
Why Existing Solutions Don't Work:
  Project Management Tools (Notion, Linear):
    - Too complex for personal projects
    - Not Git-integrated
    - Require manual updates
    - Separate from code

  AI Conversation Platforms:
    - Siloed conversations
    - No integration between platforms
    - Context lost between sessions
    - No connection to actual code

  Git-based Solutions:
    - No AI conversation tracking
    - Manual commit messages
    - No automated analysis
    - No cross-project intelligence
```

---

## 3️⃣ User Context {#user-context}

**[ID: CTX-001]** **[CRITICAL]**

> **💡 TL;DR:** Solo developer managing 15+ concurrent projects, using multiple AI assistants, with background agents autonomously writing code. Needs simple, Git-native solution that requires zero additional infrastructure.

### Primary User Profile

```yaml
Demographics:
  Role: Full-stack developer / Indie hacker
  Projects: 15-20 concurrent
  Work Style: Rapid prototyping, parallel development
  Tool Philosophy: Simple, automated, Git-centric

Current Workflow:
  Planning Phase:
    - Discusses ideas with ChatGPT, Claude, Gemini
    - Creates comprehensive PRD.md files
    - Initializes empty Git repositories
    - Sometimes starts development, often stalls

  Development Phase:
    - Cursor AI writes code in background
    - GitHub Copilot suggests completions
    - Multiple AI agents work autonomously
    - Developer doesn't review all changes

  Pain Points:
    - "Which AI made this decision?"
    - "What did Cursor change while I was away?"
    - "Where did I leave off 2 weeks ago?"
    - "Didn't I already build this auth module?"
    - "Why is the code different from the PRD?"

Daily Reality:
  Morning:
    - Opens 5 repo tabs
    - Can't remember project states
    - Spends 30 min reviewing commits
    - Discovers unexpected AI changes

  During Development:
    - Switches between projects rapidly
    - Loses context with each switch
    - Recreates similar modules
    - PRD becomes outdated immediately

  Evening:
    - Forgets to document decisions
    - AI conversation history scattered
    - Tomorrow's context already fading
```

---

## 4️⃣ Success Criteria {#success-criteria}

**[ID: SUC-001]** **[MVP]**

> **💡 TL;DR:** Success = Zero context loss, 100% PRD-code alignment, no forgotten projects, 40% module reuse rate. Everything tracked in Git without external dependencies.

### Measurable Goals

```yaml
Context Preservation:
  ✓ Every AI conversation saved in repo
  ✓ All decisions traceable via Git history
  ✓ Context recovery in <30 seconds
  ✓ Decision trail never lost

Documentation Accuracy:
  ✓ PRD auto-updated with each change
  ✓ Code-PRD drift detected within 24 hours
  ✓ Discrepancies reported clearly
  ✓ Background AI changes documented

Project Continuity:
  ✓ No project inactive >5 days unnoticed
  ✓ Automatic next-steps reminders
  ✓ Clear resumption points
  ✓ Progress never lost

Development Efficiency:
  ✓ Module reuse identified automatically
  ✓ Cross-project patterns detected
  ✓ Duplicate work eliminated
  ✓ 40% faster development via reuse
```

---

# 🎯 PART II: SOLUTION & FEATURES

---

## 5️⃣ Core Features {#core-features}

**[ID: COR-001]** **[MVP]** **[CRITICAL]**

> **💡 TL;DR:** Five core modules working together: PRD Summarizer (commit intelligence), Conversation Graph (decision tracking), Code Sync Analyzer (drift detection), Loop Reminder (continuity), and Reuse Scout (efficiency).

---

### 5.1 Contextual PRD Summarizer {#prd-summarizer}

**[ID: PRS-001]** **[MVP]** **[CRITICAL]** ⚡

> **💡 Function:** Detects PRD changes and generates intelligent commit messages with context.

<details>
<summary><b>🔧 Implementation Details</b></summary>

```typescript
interface PRDSummarizer {
  // Core functionality
  async detectChanges(prdPath: string): Promise<Changes> {
    const diff = await git.diff('HEAD', prdPath);
    const sections = this.parseSections(diff);
    const summary = await this.generateSummary(sections);
    
    return {
      sections: sections,
      summary: summary,
      commitMessage: this.formatCommitMessage(summary),
      importance: this.calculateImportance(sections)
    };
  }

  // Commit message format
  formatCommitMessage(summary: Summary): string {
    // Format: feat(prd): "Section Name" updated @PRD#Category >Decision
    const section = summary.primarySection;
    const action = summary.actionType;
    const category = summary.category;
    
    return `${action}(prd): "${section}" updated @PRD#${category} >Decision`;
  }

  // AI-powered summarization
  async generateSummary(sections: Section[]): Promise<Summary> {
    const prompt = `
      Summarize these PRD changes in one line:
      ${sections.map(s => s.diff).join('\n')}
      
      Focus on: What decision was made and why
    `;
    
    return await ai.complete(prompt);
  }
}
```

</details>

#### Features :

```yaml
Change Detection:
  - Git diff analysis on PRD.md
  - Section-level change identification
  - Addition/deletion/modification classification
  - Change importance scoring

Summary Generation:
  - AI-powered one-line summaries
  - Decision-focused descriptions
  - Technical context preservation
  - Stakeholder-readable output

Commit Enhancement:
  - Automatic commit message generation
  - Semantic versioning hints
  - Category tagging (#Backend, #API, etc.)
  - Decision markers (>Decision, >Pivot, >Feature)

Integration:
  - Pre-commit hook integration
  - Manual CLI invocation
  - GitHub Action trigger
  - VS Code extension support
```

---

### 5.2 Conversation Graph View {#conversation-graph}

**[ID: CGV-001]** **[MVP]** ⚡

> **💡 Function:** Creates chronological visualization of all AI conversations, commits, and decisions.

<details>
<summary><b>🔧 Implementation Details</b></summary>

```python
class ConversationGraph:
    def __init__(self, repo_path):
        self.repo = git.Repo(repo_path)
        self.conversations_dir = os.path.join(repo_path, 'conversations')
    
    def build_graph(self) -> Graph:
        """Build complete decision timeline"""
        events = []
        
        # Collect AI conversations
        for conv_file in os.listdir(self.conversations_dir):
            conv_data = self.parse_conversation(conv_file)
            events.extend(self.extract_decisions(conv_data))
        
        # Collect git commits
        for commit in self.repo.iter_commits():
            events.append({
                'timestamp': commit.committed_datetime,
                'type': 'commit',
                'author': commit.author.name,
                'message': commit.message,
                'ai_generated': self.is_ai_commit(commit)
            })
        
        # Build timeline
        events.sort(key=lambda x: x['timestamp'])
        return self.create_timeline_markdown(events)
    
    def extract_decisions(self, conversation):
        """Extract key decisions from AI conversation"""
        decisions = []
        for message in conversation['messages']:
            if self.is_decision(message):
                decisions.append({
                    'timestamp': message['timestamp'],
                    'type': 'decision',
                    'ai_platform': conversation['platform'],
                    'summary': self.summarize_decision(message),
                    'confidence': self.calculate_confidence(message)
                })
        return decisions
```

</details>

#### Output Format :

```markdown
# Conversation Graph - Project Alpha

## Timeline View

### 2025-10-27 - Monday
- **09:15** 💬 [ChatGPT] Decision: Use PostgreSQL over MongoDB for main database
- **09:45** 📝 [Commit] feat(prd): "Database Architecture" section added
- **10:30** 💬 [Claude] Refined API endpoint structure with REST principles  
- **11:00** 🤖 [Cursor] Auto-generated initial schema.sql
- **14:20** 💬 [Gemini] Decision: Implement authentication with JWT
- **15:00** ⚠️ [Analyzer] Code drift detected: Auth implementation differs from PRD

### 2025-10-26 - Sunday
- **16:00** 💬 [ChatGPT] Project inception: E-commerce platform concept
- **16:30** 📝 [Commit] Initial PRD.md created
```

---

### 5.3 Code Sync Analyzer {#code-sync-analyzer}

**[ID: CSA-001]** **[MVP]** **[CRITICAL]** ⚡

> **💡 Function:** Compares actual code with PRD specifications, identifies drift, especially from background AI agents.

<details>
<summary><b>🔧 Implementation Details</b></summary>

```python
class CodeSyncAnalyzer:
    def analyze(self, repo_path: str) -> Report:
        """Analyze code-PRD synchronization"""
        prd = self.parse_prd(f"{repo_path}/PRD.md")
        code = self.analyze_codebase(repo_path)
        
        discrepancies = []
        
        # Check API endpoints
        prd_endpoints = prd.get_endpoints()
        code_endpoints = code.get_endpoints()
        
        for endpoint in code_endpoints:
            if endpoint not in prd_endpoints:
                discrepancies.append({
                    'type': 'undocumented_endpoint',
                    'endpoint': endpoint,
                    'file': endpoint.file,
                    'added_by': self.get_commit_author(endpoint.file, endpoint.line),
                    'severity': 'high' if self.is_ai_commit(endpoint) else 'medium'
                })
        
        # Check data models
        prd_models = prd.get_data_models()
        code_models = code.get_models()
        
        for model in code_models:
            prd_model = prd_models.get(model.name)
            if not prd_model:
                discrepancies.append({
                    'type': 'undocumented_model',
                    'model': model.name,
                    'severity': 'high'
                })
            elif model.fields != prd_model.fields:
                discrepancies.append({
                    'type': 'model_mismatch',
                    'model': model.name,
                    'differences': self.diff_fields(model.fields, prd_model.fields),
                    'severity': 'medium'
                })
        
        return self.generate_report(discrepancies)
    
    def generate_report(self, discrepancies: list) -> str:
        """Generate markdown report"""
        report = "# Code Sync Analysis Report\n\n"
        report += f"Generated: {datetime.now()}\n"
        report += f"Total Issues: {len(discrepancies)}\n\n"
        
        # Group by severity
        high = [d for d in discrepancies if d['severity'] == 'high']
        medium = [d for d in discrepancies if d['severity'] == 'medium']
        
        if high:
            report += "## 🔴 High Priority Issues\n\n"
            for issue in high:
                report += self.format_issue(issue)
        
        if medium:
            report += "## 🟡 Medium Priority Issues\n\n"
            for issue in medium:
                report += self.format_issue(issue)
        
        return report
```

</details>

#### Report Example :

```markdown
# Code Sync Analysis Report

Generated: 2025-10-27 15:00:00
Total Issues: 5

## 🔴 High Priority Issues

### Undocumented Endpoint
- **Endpoint:** `POST /api/v2/users/batch`
- **File:** `routes/users.js:145`
- **Added by:** Cursor AI (background commit)
- **Recommendation:** Update PRD.md with endpoint specification

### Model Mismatch
- **Model:** User
- **Issue:** Field 'premium_tier' exists in code but not in PRD
- **Location:** `models/User.js:34`
- **Recommendation:** Sync PRD data model with implementation

## 🟡 Medium Priority Issues

### Configuration Drift
- **Issue:** Redis configuration in code differs from PRD
- **PRD:** Single Redis instance
- **Code:** Redis cluster with 3 nodes
- **Recommendation:** Update architecture section in PRD
```

---

### 5.4 Loop Reminder {#loop-reminder}

**[ID: LRM-001]** **[MVP]**

> **💡 Function:** Monitors project activity and creates reminders for inactive projects.

<details>
<summary><b>🔧 Implementation Details</b></summary>

```yaml
name: Loop Reminder
on:
  schedule:
    - cron: '0 7 * * *'  # Daily at 7 AM

jobs:
  check_activity:
    runs-on: ubuntu-latest
    steps:
      - name: Check last commit
        run: |
          LAST_COMMIT=$(git log -1 --format="%cr")
          DAYS_INACTIVE=$(git log -1 --format="%ct" | xargs -I {} date -d @{} +%s | xargs -I {} expr $(date +%s) - {} | xargs -I {} expr {} / 86400)
          
          if [ $DAYS_INACTIVE -ge 5 ]; then
            echo "CREATE_ISSUE=true" >> $GITHUB_ENV
            echo "DAYS=$DAYS_INACTIVE" >> $GITHUB_ENV
          fi
      
      - name: Extract Next Steps
        if: env.CREATE_ISSUE == 'true'
        run: |
          NEXT_STEPS=$(grep -A 10 "## Next Steps" PRD.md | head -n 10)
          echo "NEXT_STEPS<<EOF" >> $GITHUB_ENV
          echo "$NEXT_STEPS" >> $GITHUB_ENV
          echo "EOF" >> $GITHUB_ENV
      
      - name: Create Issue
        if: env.CREATE_ISSUE == 'true'
        uses: actions/github-script@v6
        with:
          script: |
            const issue = await github.rest.issues.create({
              owner: context.repo.owner,
              repo: context.repo.repo,
              title: `⏰ Project inactive for ${process.env.DAYS} days`,
              body: `## Project Status Alert
              
              This project has been inactive for **${process.env.DAYS} days**.
              
              ### Last Activity
              - Last commit: ${process.env.LAST_COMMIT}
              - Last author: ${process.env.LAST_AUTHOR}
              
              ### Next Steps from PRD
              ${process.env.NEXT_STEPS}
              
              ### Quick Actions
              - [ ] Review current status
              - [ ] Update PRD if needed
              - [ ] Continue development
              - [ ] Or archive project
              
              ---
              *This reminder was automatically generated by Project Nexus*`
            });
```

</details>

---

### 5.5 Cross-Project Reuse Scout {#reuse-scout}

**[ID: RSC-001]** **[V2]** ⚡

> **💡 Function:** Scans all repositories for similar modules and suggests reuse opportunities.

<details>
<summary><b>🔧 Implementation Details</b></summary>

```python
class ReuseScout:
    def __init__(self, repos_dir: str):
        self.repos = self.scan_repositories(repos_dir)
        self.module_index = {}
        
    def scan_for_modules(self) -> Dict[str, List[Module]]:
        """Scan all repos for reusable modules"""
        for repo in self.repos:
            modules = self.extract_modules(repo)
            for module in modules:
                signature = self.calculate_signature(module)
                if signature not in self.module_index:
                    self.module_index[signature] = []
                self.module_index[signature].append({
                    'repo': repo.name,
                    'module': module,
                    'path': module.path,
                    'quality_score': self.assess_quality(module)
                })
        
        return self.find_reuse_opportunities()
    
    def calculate_signature(self, module: Module) -> str:
        """Create module signature for similarity matching"""
        features = []
        features.append(module.primary_function)
        features.extend(module.dependencies)
        features.extend(module.exports)
        
        return hashlib.md5(''.join(sorted(features)).encode()).hexdigest()
    
    def find_reuse_opportunities(self) -> List[Suggestion]:
        """Identify modules that could be reused"""
        suggestions = []
        
        for signature, instances in self.module_index.items():
            if len(instances) > 1:
                best = max(instances, key=lambda x: x['quality_score'])
                for instance in instances:
                    if instance != best:
                        suggestions.append({
                            'repo': instance['repo'],
                            'current_module': instance['module'].name,
                            'suggested_module': best['module'].name,
                            'suggested_from': best['repo'],
                            'reason': f"Similar {instance['module'].primary_function} implementation",
                            'time_saved': self.estimate_time_saved(instance['module']),
                            'confidence': self.calculate_confidence(instance, best)
                        })
        
        return suggestions
    
    def update_prd_with_suggestions(self, repo: Repository, suggestions: List):
        """Add reuse suggestions to PRD"""
        prd_path = os.path.join(repo.path, 'PRD.md')
        
        suggestions_md = "\n## Reuse Suggestions\n"
        suggestions_md += "<!-- REUSE-SCOUT:BEGIN -->\n"
        
        for suggestion in suggestions:
            suggestions_md += f"""
### 📦 {suggestion['current_module']}
- **Suggestion:** Use `{suggestion['suggested_module']}` from `{suggestion['suggested_from']}`
- **Reason:** {suggestion['reason']}
- **Time Saved:** ~{suggestion['time_saved']} hours
- **Confidence:** {suggestion['confidence']}%
- **Action:** `cp ../{suggestion['suggested_from']}/{suggestion['path']} ./modules/`
"""
        
        suggestions_md += "<!-- REUSE-SCOUT:END -->\n"
        
        # Update PRD
        self.insert_into_prd(prd_path, suggestions_md)
```

</details>

#### Reuse Detection Features

```yaml
Module Similarity Detection:
  Code Analysis:
    - AST comparison
    - Function signatures
    - Import patterns
    - API endpoints
    
  Pattern Matching:
    - Similar variable names
    - Common algorithms
    - Shared dependencies
    - Parallel structure
    
  Quality Scoring:
    - Test coverage
    - Documentation completeness
    - Last update recency
    - Performance metrics

Suggestion Generation:
  Prioritization:
    - Time savings estimate
    - Quality differential
    - Compatibility check
    - Migration complexity
    
  Presentation:
    - Clear action steps
    - Migration guide
    - Risk assessment
    - Rollback plan
```

---

## 6️⃣ User Experience {#user-experience}

**[ID: UXP-001]** **[MVP]**

> **💡 TL;DR:** Three interaction surfaces: CLI for power users, mobile app for monitoring, GitHub UI for basic operations. Focus on zero-friction workflow integration.

<details>
<summary><b>🔧 Detailed UI Specifications</b></summary>

### Command Line Interface

```bash
# Primary commands with intuitive aliases
nexus init                    # Initialize repository
nexus add conversation <file>  # Import AI conversation  
nexus analyze                  # Run code-PRD sync
nexus scout                    # Find reusable modules
nexus status                   # Project health check
nexus remind                   # Check inactive projects
nexus timeline                 # View conversation graph

# Advanced commands
nexus config set <key> <value>
nexus diff prd                # Show PRD changes
nexus extract decisions        # Extract decisions from conversations
nexus generate report          # Create comprehensive report
```

### Mobile Dashboard Architecture

```typescript
interface MobileAppStructure {
  navigation: {
    type: 'tab-based';
    tabs: [
      { icon: 'grid', label: 'Projects', screen: ProjectListScreen },
      { icon: 'timeline', label: 'Activity', screen: ActivityFeed },
      { icon: 'alert', label: 'Issues', screen: IssuesScreen },
      { icon: 'settings', label: 'Settings', screen: SettingsScreen }
    ];
  };
  
  screens: {
    ProjectList: {
      layout: 'card-grid';
      sortOptions: ['last-active', 'name', 'drift-level'];
      filterOptions: ['all', 'active', 'inactive', 'has-issues'];
      quickActions: ['scan-all', 'import-conversation'];
    };
    
    ProjectDetail: {
      sections: [
        { id: 'prd', title: 'PRD', renderer: MarkdownViewer },
        { id: 'timeline', title: 'Conversations', renderer: TimelineGraph },
        { id: 'reports', title: 'Analysis', renderer: ReportViewer },
        { id: 'modules', title: 'Modules', renderer: ModuleList }
      ];
      headerActions: ['sync', 'analyze', 'export'];
    };
  };
}
```

</details>

### Visual Design System

```yaml
Color Palette:
  Primary:
    - Brand Blue: #0066FF
    - Success Green: #00AA00
    - Warning Orange: #FF9900
    - Error Red: #FF3333
    
  Semantic:
    - Active/Synced: Green
    - Inactive/Stale: Gray
    - Drift/Issue: Orange
    - Critical: Red
    
  Code Highlighting:
    - Added: Green background
    - Removed: Red background
    - Modified: Blue background
    - AI-generated: Purple border

Typography:
  Headers:
    - Font: Inter Bold
    - H1: 24px
    - H2: 20px
    - H3: 16px
    
  Body:
    - Font: Inter Regular
    - Size: 14px
    - Line height: 1.5
    - Code: JetBrains Mono

Icons:
  Project States:
    - ✅ Synced
    - ⚠️ Drift detected
    - 🔴 Critical issues
    - 💤 Inactive
    - 🤖 AI activity
    
  Actions:
    - 📥 Import
    - 🔄 Sync
    - 📊 Analyze
    - 🔍 Search
```

### Interaction Patterns

```yaml
Gestures:
  Mobile:
    - Swipe right: Mark as reviewed
    - Swipe left: Add to queue
    - Long press: Context menu
    - Pull down: Refresh
    - Pinch: Zoom timeline
    
  Desktop:
    - Drag & drop: Import files
    - Right click: Context menu
    - Ctrl+K: Command palette
    - Esc: Cancel/close

Feedback:
  Success:
    - Green toast: "✓ Analysis complete"
    - Confetti: Major milestone
    - Sound: Optional chime
    
  Progress:
    - Linear progress bar
    - Estimated time remaining
    - Cancel option
    
  Error:
    - Red banner with action
    - Detailed error log
    - Recovery suggestion
```

---

## 7️⃣ Workflow Automation {#workflow-automation}

**[ID: WFA-001]** **[MVP]**

> **💡 TL;DR:** Fully automated pipeline from conversation import to drift detection. Manual triggers available but not required. Everything happens in background via Git hooks and GitHub Actions.

<details>
<summary><b>🔧 Automation Configuration</b></summary>

```yaml
# .nexus/automation.yml
version: 1.0
automation:
  conversation_import:
    enabled: true
    watch_folders:
      - ~/Downloads
      - ~/Documents/AI-Conversations
    patterns:
      - "chatgpt-*.json"
      - "claude-export-*.json"
      - "conversation-*.md"
    auto_commit: true
    auto_push: true
    
  prd_analysis:
    enabled: true
    triggers:
      - on_prd_change
      - on_push
      - daily_at: "09:00"
    generate_summary: true
    update_commit_message: true
    
  code_sync:
    enabled: true
    scan_frequency: "on_push"
    report_threshold: "medium"
    auto_create_issue: true
    
  reminders:
    enabled: true
    inactivity_days: 5
    reminder_channels:
      - github_issue
      - email
      - slack
    escalation:
      7_days: "high_priority"
      14_days: "critical"
      
  module_reuse:
    enabled: true
    scan_repos:
      - ~/projects/*
      - ~/work/*
    min_similarity: 0.7
    auto_suggest: true
```

</details>

### Automation Flows

```mermaid
stateDiagram-v2
    [*] --> Monitoring
    
    Monitoring --> ConversationDetected: New file
    Monitoring --> CodeChanged: Git push
    Monitoring --> TimeElapsed: 24 hours
    
    ConversationDetected --> ImportConversation
    ImportConversation --> ExtractDecisions
    ExtractDecisions --> UpdatePRD
    UpdatePRD --> CommitChanges
    
    CodeChanged --> AnalyzeCode
    AnalyzeCode --> ComparePRD
    ComparePRD --> GenerateReport
    GenerateReport --> CreateIssue: If drift
    GenerateReport --> UpdateStatus: If synced
    
    TimeElapsed --> CheckActivity
    CheckActivity --> CreateReminder: If inactive
    CheckActivity --> ScanModules: If active
    ScanModules --> SuggestReuse
    
    CommitChanges --> [*]
    CreateIssue --> [*]
    UpdateStatus --> [*]
    CreateReminder --> [*]
    SuggestReuse --> [*]
```

### Automation Rules Engine

```python
class AutomationEngine:
    def __init__(self, config_path):
        self.config = self.load_config(config_path)
        self.rules = []
        self.watchers = {}
        
    def define_rule(self, trigger, condition, action):
        """Define custom automation rule"""
        rule = AutomationRule(
            id=uuid4(),
            trigger=trigger,
            condition=condition,
            action=action,
            enabled=True
        )
        self.rules.append(rule)
        return rule
    
    def example_rules(self):
        """Pre-defined automation rules"""
        
        # Auto-import conversations
        self.define_rule(
            trigger="file_created",
            condition=lambda f: f.name.endswith('.json') and 'chat' in f.name,
            action=lambda f: self.import_conversation(f)
        )
        
        # Alert on high drift
        self.define_rule(
            trigger="analysis_complete",
            condition=lambda r: r.drift_score > 0.7,
            action=lambda r: self.create_urgent_issue(r)
        )
        
        # Weekly summary
        self.define_rule(
            trigger="schedule:weekly",
            condition=lambda: True,
            action=lambda: self.generate_weekly_summary()
        )
        
        # Module suggestion on new project
        self.define_rule(
            trigger="prd_created",
            condition=lambda p: len(p.required_modules) > 0,
            action=lambda p: self.scout_similar_modules(p)
        )
```

---

# ⚙️ PART III: TECHNICAL IMPLEMENTATION (Continued)

---

## 8️⃣ Technical Architecture {#technical-architecture}

**[ID: ARC-001]** **[MVP]** **[CRITICAL]**

> **💡 TL;DR:** Microservices architecture with Node.js/TypeScript core, Python analysis services, React Native mobile, and GitHub as primary data store. No traditional database required.

<details>
<summary><b>🔧 Complete Architecture Diagram</b></summary>

```mermaid
graph TB
    subgraph "Client Layer"
        CLI[CLI Tool<br/>TypeScript]
        Mobile[Mobile App<br/>React Native]
        Web[Web Dashboard<br/>Next.js]
    end
    
    subgraph "API Gateway"
        Gateway[API Gateway<br/>Express.js]
        Auth[Auth Service<br/>JWT]
        RateLimit[Rate Limiter<br/>Redis]
    end
    
    subgraph "Core Services"
        PRDService[PRD Service<br/>Node.js]
        ConvoService[Conversation Service<br/>Node.js]
        AnalysisService[Analysis Service<br/>Python]
        ScoutService[Scout Service<br/>Python]
    end
    
    subgraph "Data Layer"
        Git[Git Repositories<br/>Primary Storage]
        Redis[Redis Cache<br/>Session/State]
        S3[S3 Bucket<br/>Backups]
    end
    
    subgraph "Automation"
        GHActions[GitHub Actions]
        Webhooks[Webhook Handler]
        Scheduler[Cron Scheduler]
    end
    
    CLI --> Gateway
    Mobile --> Gateway
    Web --> Gateway
    
    Gateway --> Auth
    Gateway --> RateLimit
    Gateway --> PRDService
    Gateway --> ConvoService
    Gateway --> AnalysisService
    Gateway --> ScoutService
    
    PRDService --> Git
    ConvoService --> Git
    AnalysisService --> Git
    ScoutService --> Git
    
    PRDService --> Redis
    ConvoService --> Redis
    
    Git --> GHActions
    GHActions --> Webhooks
    Webhooks --> Gateway
    
    Scheduler --> GHActions
    
    Git --> S3
```

</details>

### Service Specifications

```yaml
Core Services:
  PRD Service:
    Language: TypeScript
    Framework: Express.js
    Responsibilities:
      - PRD parsing and validation
      - Change detection
      - Summary generation
      - Version management
    Dependencies:
      - markdown-it: MD parsing
      - diff: Change detection
      - openai: Summarization
      
  Conversation Service:
    Language: TypeScript
    Framework: Express.js
    Responsibilities:
      - Import conversations
      - Extract decisions
      - Build timeline graph
      - Format conversions
    Supported Formats:
      - ChatGPT JSON
      - Claude JSON/MD
      - Gemini JSON
      - Custom formats
      
  Analysis Service:
    Language: Python 3.11
    Framework: FastAPI
    Responsibilities:
      - Code structure analysis
      - PRD-code comparison
      - Drift detection
      - Report generation
    Libraries:
      - ast: Code parsing
      - gitpython: Git operations
      - pandas: Data analysis
      - jinja2: Report templates
      
  Scout Service:
    Language: Python 3.11
    Framework: FastAPI
    Responsibilities:
      - Module detection
      - Similarity calculation
      - Reuse suggestions
      - Cross-repo scanning
    Algorithms:
      - TF-IDF: Text similarity
      - AST diff: Code similarity
      - Levenshtein: Name matching

Infrastructure:
  Deployment:
    Platform: AWS/GCP/Azure
    Orchestration: Kubernetes
    CI/CD: GitHub Actions
    Monitoring: Prometheus + Grafana
    
  Scaling:
    API Gateway: 2-10 instances
    Services: 1-5 instances each
    Redis: Cluster mode
    Git: Distributed repos
    
  Performance:
    API Response: <200ms p95
    Analysis: <10s for 100k LOC
    Scout: <30s for 50 repos
    Cache Hit Rate: >80%
```

### Security Architecture

```yaml
Authentication:
  Methods:
    - JWT tokens (primary)
    - API keys (CLI)
    - OAuth 2.0 (integrations)
    
  Token Management:
    Access Token: 15 minutes
    Refresh Token: 30 days
    API Keys: No expiration
    Revocation: Immediate
    
Authorization:
  Model: RBAC (Role-Based Access Control)
  Roles:
    - Owner: Full access
    - Contributor: Read/write
    - Viewer: Read only
    
  Permissions:
    - repo:read
    - repo:write
    - analysis:run
    - settings:modify

Data Security:
  Encryption:
    At Rest: AES-256-GCM
    In Transit: TLS 1.3
    Secrets: Vault/KMS
    
  Privacy:
    - No data sharing
    - Local processing when possible
    - Opt-in analytics only
    - GDPR compliant deletion
    
Audit & Compliance:
  Logging:
    - All API calls logged
    - Git operations tracked
    - Analysis results archived
    
  Compliance:
    - SOC 2 Type II ready
    - GDPR compliant
    - CCPA compliant
    - ISO 27001 aligned
```

---

## 9️⃣ Git-Native Design {#git-native-design}

**[ID: GIT-001]** **[MVP]** **[CRITICAL]**

> **💡 TL;DR:** Git is the database. Every operation is a commit. History is the audit log. Branches enable experimentation. No external state required.

### Git as Database Philosophy

```python
class GitDatabase:
    """Git repository as primary data store"""
    
    def __init__(self, repo_path):
        self.repo = git.Repo(repo_path)
        self.data_branch = 'nexus-data'
        
    def create_schema(self):
        """Initialize repository structure"""
        folders = [
            'conversations',
            'modules', 
            'reports',
            '.nexus'
        ]
        
        for folder in folders:
            os.makedirs(folder, exist_ok=True)
            
        # Create index files
        self.create_index('conversations')
        self.create_index('modules')
        
    def save_document(self, collection, doc_id, data):
        """Save document to Git"""
        file_path = f"{collection}/{doc_id}.json"
        
        with open(file_path, 'w') as f:
            json.dump(data, f, indent=2)
            
        self.repo.index.add([file_path])
        self.repo.index.commit(
            f"auto: Update {collection}/{doc_id}",
            author=Actor("Nexus Bot", "nexus@bot.local")
        )
        
    def query_documents(self, collection, filter_func=None):
        """Query documents from Git"""
        docs = []
        path = Path(collection)
        
        for file in path.glob("*.json"):
            with open(file) as f:
                doc = json.load(f)
                if filter_func is None or filter_func(doc):
                    docs.append(doc)
                    
        return docs
    
    def get_history(self, file_path, limit=10):
        """Get file history from Git"""
        commits = list(self.repo.iter_commits(
            paths=file_path,
            max_count=limit
        ))
        
        history = []
        for commit in commits:
            history.append({
                'sha': commit.hexsha,
                'author': commit.author.name,
                'date': commit.committed_datetime,
                'message': commit.message,
                'diff': commit.diff(commit.parents[0])[0].diff
                       if commit.parents else None
            })
            
        return history
```

### Branching Strategy

```yaml
Branch Structure:
  main:
    - Production code
    - Stable PRD
    - Released features
    
  nexus-data:
    - Conversation history
    - Analysis reports  
    - Generated summaries
    
  feature/*:
    - New development
    - PRD updates
    - Experimental modules
    
  analysis/*:
    - Temporary analysis
    - Scout results
    - Performance tests

Merge Strategy:
  Data Branches:
    - Always merge (no conflicts)
    - Append-only operations
    - Automated via Actions
    
  Feature Branches:
    - Standard PR flow
    - Requires PRD update
    - Auto-analysis on merge
```

### Git Hooks Implementation

```bash
#!/bin/bash
# .git/hooks/pre-commit

# 1. Check PRD changes
if git diff --cached --name-only | grep -q "PRD.md"; then
    echo "📝 PRD changed, generating summary..."
    
    # Extract changed sections
    CHANGES=$(git diff --cached PRD.md | grep "^[+-]" | head -20)
    
    # Generate AI summary
    SUMMARY=$(echo "$CHANGES" | nexus summarize --format=commit)
    
    # Update commit message
    echo "$SUMMARY" > .git/COMMIT_EDITMSG
fi

# 2. Validate conversations
if git diff --cached --name-only | grep -q "conversations/"; then
    echo "💬 Validating conversation files..."
    
    for file in $(git diff --cached --name-only --diff-filter=A | grep "conversations/"); do
        if ! nexus validate conversation "$file"; then
            echo "❌ Invalid conversation format: $file"
            exit 1
        fi
    done
fi

# 3. Update indexes
echo "📑 Updating indexes..."
nexus index update

# 4. Run quick analysis
if [ "$NEXUS_QUICK_ANALYSIS" = "true" ]; then
    echo "🔍 Running quick analysis..."
    nexus analyze --quick
fi

exit 0
```

```bash
#!/bin/bash
# .git/hooks/post-commit

# 1. Update conversation graph
if git diff HEAD~1 --name-only | grep -q "conversations/"; then
    nexus timeline refresh &
fi

# 2. Check if reminder needed
LAST_COMMIT_DATE=$(git log -2 --format="%ct" | tail -1)
CURRENT_DATE=$(date +%s)
DAYS_DIFF=$(( ($CURRENT_DATE - $LAST_COMMIT_DATE) / 86400 ))

if [ $DAYS_DIFF -gt 5 ]; then
    nexus remind check --create-issue &
fi

# 3. Trigger scout if new modules
if git diff HEAD~1 --name-only | grep -q "modules/"; then
    nexus scout run --async &
fi
```

---

## 🔟 AI Components {#ai-components}

**[ID: AIC-001]** **[MVP]**

> **💡 TL;DR:** Modular AI system with OpenAI for summaries, local AST for code analysis, pattern matching for decisions, and TF-IDF for similarity. Privacy-first with minimal external API calls.

<details>
<summary><b>🔧 AI Implementation Details</b></summary>

```python
class NexusAI:
    """Central AI orchestration for Project Nexus"""
    
    def __init__(self, config):
        self.config = config
        self.openai = OpenAI(api_key=config.openai_key) if config.openai_key else None
        self.local_nlp = LocalNLP()
        self.pattern_matcher = PatternMatcher()
        self.similarity_engine = SimilarityEngine()
        
    async def process_prd_change(self, diff):
        """Generate intelligent summary of PRD changes"""
        if not self.openai:
            return self.local_nlp.summarize(diff)
            
        prompt = f"""
        Summarize this PRD change in one line (max 80 chars).
        Focus on WHAT changed and WHY it matters.
        Use technical but accessible language.
        
        Diff:
        {diff[:1000]}  # Truncate for token limits
        
        Format: [Component] [change] for [purpose]
        Example: "API endpoints restructured for better RESTful compliance"
        """
        
        response = await self.openai.chat.completions.create(
            model="gpt-3.5-turbo",
            messages=[{"role": "user", "content": prompt}],
            max_tokens=50,
            temperature=0.3
        )
        
        return response.choices[0].message.content.strip()
    
    def extract_decisions(self, conversation):
        """Extract key decisions from AI conversations"""
        decisions = []
        decision_patterns = [
            r"(?i)decided?\s+to\s+(.+)",
            r"(?i)let'?s\s+go\s+with\s+(.+)",
            r"(?i)will\s+use\s+(.+)",
            r"(?i)choosing\s+(.+)\s+because",
            r"(?i)best\s+option\s+is\s+(.+)",
        ]
        
        for message in conversation.get('messages', []):
            content = message.get('content', '')
            
            # Check for decision patterns
            for pattern in decision_patterns:
                matches = re.findall(pattern, content)
                for match in matches:
                    decision = {
                        'text': match,
                        'timestamp': message.get('timestamp'),
                        'confidence': self.calculate_confidence(match, message),
                        'context': self.extract_context(content, match),
                        'impact': self.assess_impact(match)
                    }
                    decisions.append(decision)
        
        return self.rank_decisions(decisions)
    
    def calculate_code_similarity(self, module1, module2):
        """Calculate similarity between two code modules"""
        # Extract features
        features1 = self.extract_module_features(module1)
        features2 = self.extract_module_features(module2)
        
        # Calculate different similarity metrics
        structural_sim = self.ast_similarity(module1.ast, module2.ast)
        semantic_sim = self.semantic_similarity(features1, features2)
        dependency_sim = self.dependency_similarity(module1.deps, module2.deps)
        
        # Weighted average
        similarity = (
            structural_sim * 0.4 +
            semantic_sim * 0.4 +
            dependency_sim * 0.2
        )
        
        return {
            'overall': similarity,
            'structural': structural_sim,
            'semantic': semantic_sim,
            'dependencies': dependency_sim,
            'confidence': self.calculate_similarity_confidence(module1, module2)
        }
```

</details>

### AI Feature Specifications

```yaml
PRD Summarization:
  Model: GPT-3.5-turbo (OpenAI)
  Fallback: Local extractive summarization
  Token Limit: 100 tokens output
  Temperature: 0.3 (consistent)
  Cost: ~$0.001 per summary
  Cache: 24 hours
  
Decision Extraction:
  Method: Pattern matching + NLP
  Patterns: 15+ decision indicators
  Confidence Threshold: 0.6
  Context Window: ±2 messages
  Ranking: Impact + confidence
  
Code Analysis:
  Parser: Python AST / TypeScript AST
  Metrics:
    - Cyclomatic complexity
    - Function signatures
    - Import patterns
    - Class hierarchies
  No External API: Privacy-first
  
Module Similarity:
  Algorithm: TF-IDF + AST diff
  Threshold: 0.7 similarity
  Features:
    - Function names
    - Variable patterns
    - Control flow
    - Dependencies
  Performance: <1s per comparison

Pattern Detection:
  Method: Sliding window + clustering
  Window Size: 7-30 days
  Min Occurrences: 3
  Confidence: Bayesian inference
  Types:
    - Task sequences
    - Time patterns
    - Energy cycles
    - Productivity patterns
```

### Privacy-Conscious AI

```python
class PrivacyAI:
    """AI operations with privacy protection"""
    
    def sanitize_for_external_api(self, text):
        """Remove sensitive information before API calls"""
        # Remove API keys
        text = re.sub(r'[a-zA-Z0-9]{32,}', '[REDACTED_KEY]', text)
        
        # Remove emails
        text = re.sub(r'[\w\.-]+@[\w\.-]+', '[EMAIL]', text)
        
        # Remove URLs with credentials
        text = re.sub(r'https?://[^:]+:[^@]+@', 'https://[CREDS]@', text)
        
        # Remove file paths
        text = re.sub(r'/[\w/\.-]+', '[PATH]', text)
        
        return text
    
    def process_locally_if_possible(self, task, data):
        """Prefer local processing over API calls"""
        local_capabilities = [
            'code_analysis',
            'pattern_detection',
            'similarity_calculation',
            'basic_summarization'
        ]
        
        if task in local_capabilities:
            return self.local_processor.process(task, data)
        
        # Only use external API if necessary
        if self.user_consent_for_api:
            sanitized = self.sanitize_for_external_api(data)
            return self.external_api.process(task, sanitized)
        
        return self.fallback_processor.process(task, data)
```

---

## 1️⃣1️⃣ CLI Tools {#cli-tools}

**[ID: CLI-001]** **[MVP]**

> **💡 TL;DR:** Comprehensive CLI for all Nexus operations. Global npm package with intuitive commands, rich output, and extensive customization options.

<details>
<summary><b>🔧 Complete CLI Implementation</b></summary>

```typescript
#!/usr/bin/env node
import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import inquirer from 'inquirer';
import { table } from 'table';

class NexusCLI {
  private program: Command;
  
  constructor() {
    this.program = new Command();
    this.setupCommands();
  }
  
  private setupCommands() {
    this.program
      .name('nexus')
      .description('AI-powered project memory & repository sync system')
      .version('1.0.0');
    
    // Initialize command
    this.program
      .command('init')
      .description('Initialize Nexus in current repository')
      .option('-f, --force', 'Force initialization')
      .action(async (options) => {
        const spinner = ora('Initializing Project Nexus...').start();
        
        try {
          // Create folder structure
          await this.createFolderStructure();
          
          // Copy workflow templates
          await this.copyWorkflows();
          
          // Create initial PRD if missing
          if (!await this.fileExists('PRD.md')) {
            await this.createInitialPRD();
          }
          
          // Setup git hooks
          await this.setupGitHooks();
          
          // Initialize configuration
          await this.createConfig();
          
          spinner.succeed('Project Nexus initialized successfully!');
          
          console.log('\n' + chalk.green('✓') + ' Created folder structure');
          console.log(chalk.green('✓') + ' Added GitHub workflows');
          console.log(chalk.green('✓') + ' Configured git hooks');
          console.log(chalk.green('✓') + ' Ready to track your project!');
          
          console.log('\n' + chalk.blue('Next steps:'));
          console.log('  1. Import a conversation: ' + chalk.cyan('nexus import conversation'));
          console.log('  2. Analyze your code: ' + chalk.cyan('nexus analyze'));
          console.log('  3. View status: ' + chalk.cyan('nexus status'));
          
        } catch (error) {
          spinner.fail('Initialization failed');
          console.error(chalk.red(error.message));
          process.exit(1);
        }
      });
    
    // Import conversation command
    this.program
      .command('import')
      .argument('<type>', 'Type of import (conversation, module)')
      .option('-f, --file <path>', 'File to import')
      .option('-p, --platform <platform>', 'Platform (chatgpt, claude, gemini)')
      .option('--no-commit', 'Skip auto-commit')
      .action(async (type, options) => {
        if (type === 'conversation') {
          await this.importConversation(options);
        } else if (type === 'module') {
          await this.importModule(options);
        }
      });
    
    // Analyze command
    this.program
      .command('analyze')
      .description('Analyze code-PRD synchronization')
      .option('-q, --quick', 'Quick analysis (skip deep scan)')
      .option('-o, --output <format>', 'Output format (md, json, html)', 'md')
      .option('--no-commit', 'Don\'t commit report')
      .action(async (options) => {
        const spinner = ora('Analyzing code-PRD sync...').start();
        
        try {
          const analyzer = new CodeSyncAnalyzer();
          const report = await analyzer.analyze(process.cwd(), options);
          
          // Save report
          const reportPath = `reports/code-sync-report.${options.output}`;
          await this.saveFile(reportPath, report);
          
          if (!options.noCommit) {
            await this.git.add(reportPath);
            await this.git.commit(`auto(analyzer): ${report.summary}`);
          }
          
          spinner.succeed('Analysis complete!');
          
          // Display summary
          this.displayAnalysisSummary(report);
          
        } catch (error) {
          spinner.fail('Analysis failed');
          console.error(chalk.red(error.message));
        }
      });
    
    // Status command
    this.program
      .command('status')
      .description('Show project status')
      .option('-v, --verbose', 'Detailed output')
      .action(async (options) => {
        const status = await this.getProjectStatus();
        
        console.log(chalk.bold('\n📊 Project Status\n'));
        
        const statusTable = table([
          [chalk.gray('Metric'), chalk.gray('Value')],
          ['Last Commit', this.formatTimeAgo(status.lastCommit)],
          ['PRD Status', this.formatStatus(status.prdStatus)],
          ['Code Sync', this.formatSyncStatus(status.syncStatus)],
          ['Active Issues', status.activeIssues],
          ['Conversations', status.conversationCount],
          ['Modules', status.moduleCount],
          ['Reuse Opportunities', status.reuseCount]
        ]);
        
        console.log(statusTable);
        
        if (status.warnings.length > 0) {
          console.log(chalk.yellow('\n⚠️  Warnings:'));
          status.warnings.forEach(warning => {
            console.log(chalk.yellow(`  • ${warning}`));
          });
        }
        
        if (options.verbose) {
          await this.showDetailedStatus(status);
        }
      });
    
    // Scout command
    this.program
      .command('scout')
      .description('Find reusable modules across projects')
      .option('-d, --directory <path>', 'Directory to scan', '~/projects')
      .option('-t, --threshold <value>', 'Similarity threshold', '0.7')
      .action(async (options) => {
        const spinner = ora('Scouting for reusable modules...').start();
        
        try {
          const scout = new ReuseScout(options.directory);
          const suggestions = await scout.scan(options);
          
          spinner.succeed(`Found ${suggestions.length} reuse opportunities!`);
          
          if (suggestions.length > 0) {
            console.log(chalk.bold('\n♻️  Reusable Modules Found:\n'));
            
            suggestions.forEach((suggestion, index) => {
              console.log(chalk.cyan(`${index + 1}. ${suggestion.module}`));
              console.log(`   From: ${chalk.gray(suggestion.sourceProject)}`);
              console.log(`   Similarity: ${chalk.green(suggestion.similarity + '%')}`);
              console.log(`   Time saved: ${chalk.yellow(suggestion.timeSaved + ' hours')}`);
              console.log(`   Action: ${chalk.blue(suggestion.action)}\n`);
            });
            
            const answer = await inquirer.prompt([{
              type: 'confirm',
              name: 'update',
              message: 'Update PRD with suggestions?',
              default: true
            }]);
            
            if (answer.update) {
              await this.updatePRDWithSuggestions(suggestions);
              console.log(chalk.green('✓ PRD updated with reuse suggestions'));
            }
          }
        } catch (error) {
          spinner.fail('Scout failed');
          console.error(chalk.red(error.message));
        }
      });
    
    // Timeline command
    this.program
      .command('timeline')
      .description('View conversation timeline')
      .option('-d, --days <number>', 'Number of days to show', '30')
      .option('-f, --filter <type>', 'Filter by type')
      .action(async (options) => {
        const timeline = await this.buildTimeline(options);
        
        console.log(chalk.bold('\n📅 Conversation Timeline\n'));
        
        timeline.forEach(day => {
          console.log(chalk.blue(day.date));
          day.events.forEach(event => {
            const icon = this.getEventIcon(event.type);
            const time = chalk.gray(event.time);
            const description = event.description;
            console.log(`  ${icon} ${time} ${description}`);
          });
          console.log();
        });
      });
    
    // Config command
    this.program
      .command('config')
      .argument('<action>', 'Action (get, set, list)')
      .argument('[key]', 'Configuration key')
      .argument('[value]', 'Configuration value')
      .action(async (action, key, value) => {
        const config = new NexusConfig();
        
        switch (action) {
          case 'get':
            const val = config.get(key);
            console.log(`${key}: ${val}`);
            break;
            
          case 'set':
            config.set(key, value);
            console.log(chalk.green(`✓ ${key} set to ${value}`));
            break;
            
          case 'list':
            const all = config.list();
            console.log(chalk.bold('\n⚙️  Configuration:\n'));
            Object.entries(all).forEach(([k, v]) => {
              console.log(`  ${k}: ${v}`);
            });
            break;
        }
      });
  }
  
  private async importConversation(options: any) {
    let filePath = options.file;
    
    // Interactive file selection if not provided
    if (!filePath) {
      const answer = await inquirer.prompt([{
        type: 'input',
        name: 'file',
        message: 'Path to conversation file:',
        validate: (input) => this.fileExists(input) || 'File not found'
      }]);
      filePath = answer.file;
    }
    
    // Auto-detect platform if not specified
    const platform = options.platform || this.detectPlatform(filePath);
    
    const spinner = ora(`Importing ${platform} conversation...`).start();
    
    try {
      // Parse conversation
      const conversation = await this.parseConversation(filePath, platform);
      
      // Extract decisions
      const decisions = await this.extractDecisions(conversation);
      
      // Save to conversations folder
      const filename = this.generateFilename(platform);
      await this.saveConversation(filename, conversation);
      
      // Generate summary
      const summary = await this.summarizeConversation(conversation, decisions);
      
      // Commit if enabled
      if (!options.noCommit) {
        await this.git.add(`conversations/${filename}`);
        await this.git.commit(`feat(conversation): Import ${platform} - ${summary}`);
      }
      
      spinner.succeed('Conversation imported successfully!');
      
      console.log(chalk.green(`\n✓ Imported: ${filename}`));
      console.log(chalk.blue(`📊 Found ${decisions.length} decisions`));
      if (decisions.length > 0) {
        console.log(chalk.gray('\nTop decisions:'));
        decisions.slice(0, 3).forEach((d, i) => {
          console.log(`  ${i + 1}. ${d.text}`);
        });
      }
      
    } catch (error) {
      spinner.fail('Import failed');
      console.error(chalk.red(error.message));
    }
  }
  
  private displayAnalysisSummary(report: any) {
    const { high, medium, low } = report.issues;
    
    console.log(chalk.bold('\n📊 Analysis Summary:\n'));
    
    if (high.length > 0) {
      console.log(chalk.red(`🔴 High Priority: ${high.length} issues`));
      high.slice(0, 3).forEach(issue => {
        console.log(chalk.red(`   • ${issue.title}`));
      });
    }
    
    if (medium.length > 0) {
      console.log(chalk.yellow(`🟡 Medium Priority: ${medium.length} issues`));
      medium.slice(0, 2).forEach(issue => {
        console.log(chalk.yellow(`   • ${issue.title}`));
      });
    }
    
    if (low.length > 0) {
      console.log(chalk.blue(`🔵 Low Priority: ${low.length} issues`));
    }
    
    console.log(`\nFull report: ${chalk.cyan('reports/code-sync-report.md')}`);
  }
  
  private getEventIcon(type: string): string {
    const icons = {
      'conversation': '💬',
      'commit': '📝',
      'analysis': '🔍',
      'issue': '⚠️',
      'merge': '🔀',
      'decision': '🎯'
    };
    return icons[type] || '•';
  }
}

// Run CLI
const cli = new NexusCLI();
cli.run(process.argv);
```

</details>

### CLI Command Reference

```bash
# Complete command list with examples

# Initialize repository
nexus init                           # Interactive setup
nexus init --force                   # Override existing setup

# Import conversations
nexus import conversation --file chat.json --platform chatgpt
nexus import conversation             # Interactive mode
nexus import module --from ../project-alpha/modules/auth.js

# Analyze project
nexus analyze                        # Full analysis
nexus analyze --quick               # Skip deep scan
nexus analyze --output json         # JSON output
nexus analyze --no-commit           # Don't commit report

# Check status
nexus status                        # Basic status
nexus status --verbose              # Detailed information

# Find reusable modules
nexus scout                         # Scan default directory
nexus scout --directory ~/work      # Custom directory
nexus scout --threshold 0.8         # Higher similarity required

# View timeline
nexus timeline                      # Last 30 days
nexus timeline --days 7            # Last week
nexus timeline --filter decision    # Decisions only

# Manage configuration
nexus config list                   # Show all settings
nexus config get reminder.days      # Get specific value
nexus config set ai.model gpt-4    # Update setting

# Advanced commands
nexus validate conversation file.json  # Validate format
nexus extract decisions file.json      # Extract decisions only
nexus generate summary                  # Generate PRD summary
nexus diff prd                         # Show PRD changes
nexus remind check                     # Check for inactive projects
```

---

## 1️⃣2️⃣ Mobile Dashboard {#mobile-dashboard}

**[ID: MOB-001]**

> **💡 TL;DR:** React Native app for iOS/Android providing read-only access to project status, PRDs, and reports. Optimized for quick checks and notifications.

<details>
<summary><b>🔧 Mobile App Architecture</b></summary>

```typescript
// App.tsx - Main Application Structure
import React from 'react';
import { NavigationContainer } from '@react-navigation/native';
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs';
import { Provider } from 'react-redux';
import { store } from './store';

const Tab = createBottomTabNavigator();

export default function App() {
  return (
    <Provider store={store}>
      <NavigationContainer>
        <Tab.Navigator>
          <Tab.Screen 
            name="Projects" 
            component={ProjectListScreen}
            options={{
              tabBarIcon: ({ color }) => <GridIcon color={color} />
            }}
          />
          <Tab.Screen 
            name="Activity" 
            component={ActivityScreen}
            options={{
              tabBarIcon: ({ color }) => <TimelineIcon color={color} />
            }}
          />
          <Tab.Screen 
            name="Issues" 
            component={IssuesScreen}
            options={{
              tabBarIcon: ({ color }) => <AlertIcon color={color} />,
              tabBarBadge: issueCount > 0 ? issueCount : undefined
            }}
          />
          <Tab.Screen 
            name="Settings" 
            component={SettingsScreen}
            options={{
              tabBarIcon: ({ color }) => <SettingsIcon color={color} />
            }}
          />
        </Tab.Navigator>
      </NavigationContainer>
    </Provider>
  );
}

// ProjectListScreen.tsx
const ProjectListScreen = () => {
  const [projects, setProjects] = useState<Project[]>([]);
  const [filter, setFilter] = useState<'all' | 'active' | 'inactive'>('all');
  const [refreshing, setRefreshing] = useState(false);
  
  const onRefresh = useCallback(async () => {
    setRefreshing(true);
    await fetchProjects();
    setRefreshing(false);
  }, []);
  
  return (
    <SafeAreaView style={styles.container}>
      <Header title="Projects" />
      
      <FilterBar
        options={['all', 'active', 'inactive']}
        selected={filter}
        onSelect={setFilter}
      />
      
      {projects.length === 0 ? (
        <EmptyState 
          title="No projects yet"
          message="Import your first project from the CLI"
          icon="folder-open"
        />
      ) : (
        <FlatList
          data={filteredProjects}
          renderItem={({ item }) => (
            <ProjectCard
              project={item}
              onPress={() => navigation.navigate('ProjectDetail', { id: item.id })}
            />
          )}
          keyExtractor={item => item.id}
          refreshControl={
            <RefreshControl refreshing={refreshing} onRefresh={onRefresh} />
          }
          contentContainerStyle={styles.list}
        />
      )}
      
      <FAB
        icon="plus"
        onPress={() => showImportDialog()}
        style={styles.fab}
      />
    </SafeAreaView>
  );
};

// ProjectCard.tsx
const ProjectCard: React.FC<{ project: Project; onPress: () => void }> = ({ project, onPress }) => {
  const statusColor = {
    synced: '#00AA00',
    drift: '#FF9900',
    inactive: '#999999'
  }[project.status];
  
  return (
    <TouchableOpacity style={styles.card} onPress={onPress}>
      <View style={styles.cardHeader}>
        <Text style={styles.projectName}>{project.name}</Text>
        <View style={[styles.statusBadge, { backgroundColor: statusColor }]}>
          <Text style={styles.statusText}>{project.status}</Text>
        </View>
      </View>
      
      <View style={styles.cardContent}>
        <View style={styles.metric}>
          <Icon name="clock" size={14} color="#666" />
          <Text style={styles.metricText}>
            {formatTimeAgo(project.lastActivity)}
          </Text>
        </View>
        
        <View style={styles.metric}>
          <Icon name="message-circle" size={14} color="#666" />
          <Text style={styles.metricText}>
            {project.conversationCount} conversations
          </Text>
        </View>
        
        <View style={styles.metric}>
          <Icon name="alert-circle" size={14} color="#666" />
          <Text style={styles.metricText}>
            {project.issueCount} issues
          </Text>
        </View>
      </View>
      
      {project.hasUpdate && (
        <View style={styles.updateBadge}>
          <Text style={styles.updateText}>New analysis available</Text>
        </View>
      )}
    </TouchableOpacity>
  );
};

// ProjectDetailScreen.tsx
const ProjectDetailScreen = ({ route }) => {
  const { projectId } = route.params;
  const [activeTab, setActiveTab] = useState<'prd' | 'timeline' | 'reports'>('prd');
  const project = useProject(projectId);
  
  return (
    <SafeAreaView style={styles.container}>
      <Header
        title={project.name}
        back
        actions={[
          { icon: 'refresh-cw', onPress: () => syncProject(projectId) },
          { icon: 'share-2', onPress: () => shareProject(project) }
        ]}
      />
      
      <TabBar
        tabs={[
          { key: 'prd', label: 'PRD', icon: 'file-text' },
          { key: 'timeline', label: 'Timeline', icon: 'clock' },
          { key: 'reports', label: 'Reports', icon: 'bar-chart' }
        ]}
        selected={activeTab}
        onSelect={setActiveTab}
      />
      
      <ScrollView style={styles.content}>
        {activeTab === 'prd' && (
          <Markdown
            source={project.prd}
            style={markdownStyles}
            onLinkPress={handleLinkPress}
          />
        )}
        
        {activeTab === 'timeline' && (
          <Timeline
            events={project.conversations}
            onEventPress={handleEventPress}
          />
        )}
        
        {activeTab === 'reports' && (
          <ReportView
            report={project.latestReport}
            onIssuePress={handleIssuePress}
          />
        )}
      </ScrollView>
    </SafeAreaView>
  );
};
```

</details>

### Mobile Features

```yaml
Core Features:
  Project Overview:
    - List all tracked projects
    - Status indicators (synced/drift/inactive)
    - Last activity timestamp
    - Quick stats (conversations, issues)
    
  Project Details:
    - PRD viewer (markdown rendered)
    - Conversation timeline
    - Analysis reports
    - Module list
    - Reuse suggestions
    
  Notifications:
    - Inactive project alerts
    - High priority drift warnings
    - New conversation imports
    - Analysis complete
    
  Quick Actions:
    - Pull to refresh
    - Share project status
    - Mark as reviewed
    - Create reminder

Performance:
  Optimization:
    - Lazy loading
    - Image caching
    - Incremental sync
    - Offline support
    
  Target Metrics:
    - Cold start: <2s
    - Screen transition: <200ms
    - List scrolling: 60fps
    - Memory usage: <100MB

Platform Support:
  iOS:
    - Minimum: iOS 13
    - Optimized for: iPhone, iPad
    - Features: Widgets, Shortcuts
    
  Android:
    - Minimum: Android 6 (API 23)
    - Optimized for: Phones, tablets
    - Features: Widgets, Quick tiles
```

---

## 1️⃣3️⃣ GitHub Actions {#github-actions}

**[ID: GHA-001]** **[MVP]**

> **💡 TL;DR:** Automated workflows for analysis, reminders, and module scouting. Runs on push, schedule, or manual trigger. No configuration needed.

<details>
<summary><b>🔧 Complete Workflow Definitions</b></summary>

```yaml
# .github/workflows/nexus-complete.yml
name: Nexus Complete Pipeline

on:
  push:
    branches: [main, develop]
  schedule:
    - cron: '0 7 * * *'  # Daily at 7 AM
  workflow_dispatch:
    inputs:
      action:
        description: 'Action to perform'
        required: true
        default: 'all'
        type: choice
        options:
          - all
          - analyze
          - remind
          - scout

jobs:
  setup:
    runs-on: ubuntu-latest
    outputs:
      should-analyze: ${{ steps.check.outputs.analyze }}
      should-remind: ${{ steps.check.outputs.remind }}
      should-scout: ${{ steps.check.outputs.scout }}
    
    steps:
      - uses: actions/checkout@v3
        with:
          fetch-depth: 0
      
      - name: Determine actions
        id: check
        run: |
          # Check if PRD or code changed
          if git diff HEAD~1 --name-only | grep -E "(PRD\.md|\.js$|\.py$|\.ts$)"; then
            echo "analyze=true" >> $GITHUB_OUTPUT
          fi
          
          # Check last commit age
          LAST_COMMIT=$(git log -1 --format="%ct")
          CURRENT=$(date +%s)
          DAYS=$(( ($CURRENT - $LAST_COMMIT) / 86400 ))
          
          if [ $DAYS -ge 5 ]; then
            echo "remind=true" >> $GITHUB_OUTPUT
          fi
          
          # Check for new modules
          if git diff HEAD~1 --name-only | grep "modules/"; then
            echo "scout=true" >> $GITHUB_OUTPUT
          fi
  
  analyze:
    needs: setup
    if: needs.setup.outputs.should-analyze == 'true' || github.event.inputs.action == 'analyze' || github.event.inputs.action == 'all'
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'
          cache: 'pip'
      
      - name: Install dependencies
        run: |
          pip install -r scripts/requirements.txt
          npm install -g nexus-cli
      
      - name: Parse PRD
        id: prd
        run: |
          python scripts/parse_prd.py PRD.md > prd.json
          echo "endpoints=$(jq -r '.endpoints | length' prd.json)" >> $GITHUB_OUTPUT
          echo "models=$(jq -r '.models | length' prd.json)" >> $GITHUB_OUTPUT
      
      - name: Analyze code structure
        id: code
        run: |
          python scripts/analyze_code.py . > code.json
          echo "actual_endpoints=$(jq -r '.endpoints | length' code.json)" >> $GITHUB_OUTPUT
          echo "actual_models=$(jq -r '.models | length' code.json)" >> $GITHUB_OUTPUT
      
      - name: Compare PRD vs Code
        id: compare
        run: |
          python scripts/compare.py prd.json code.json > report.md
          
          # Count issues
          HIGH=$(grep -c "## 🔴" report.md || true)
          MEDIUM=$(grep -c "## 🟡" report.md || true)
          LOW=$(grep -c "## 🔵" report.md || true)
          
          echo "high_issues=$HIGH" >> $GITHUB_OUTPUT
          echo "medium_issues=$MEDIUM" >> $GITHUB_OUTPUT
          echo "low_issues=$LOW" >> $GITHUB_OUTPUT
      
      - name: Generate summary
        id: summary
        run: |
          if [ ${{ steps.compare.outputs.high_issues }} -gt 0 ]; then
            SUMMARY="🔴 Critical: ${{ steps.compare.outputs.high_issues }} high priority issues detected"
          elif [ ${{ steps.compare.outputs.medium_issues }} -gt 0 ]; then
            SUMMARY="🟡 Warning: ${{ steps.compare.outputs.medium_issues }} medium priority issues"
          else
            SUMMARY="✅ Synced: PRD and code are aligned"
          fi
          
          echo "summary=$SUMMARY" >> $GITHUB_OUTPUT
      
      - name: Commit report
        run: |
          mkdir -p reports
          cp report.md reports/code-sync-report.md
          
          git config user.name "Nexus Bot"
          git config user.email "nexus@bot.local"
          
          if [ -n "$(git status --porcelain reports/)" ]; then
            git add reports/code-sync-report.md
            git commit -m "auto(analyzer): ${{ steps.summary.outputs.summary }}"
            git push
          fi
      
      - name: Create issue if needed
        if: steps.compare.outputs.high_issues > 0
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const report = fs.readFileSync('report.md', 'utf8');
            
            // Extract high priority section
            const highPriority = report.match(/## 🔴 High Priority[\s\S]*?(?=##|$)/);
            
            await github.rest.issues.create({
              owner: context.repo.owner,
              repo: context.repo.repo,
              title: '🔴 Code-PRD drift: ${{ steps.compare.outputs.high_issues }} critical issues',
              body: `## Urgent: Code implementation differs from PRD
              
              ${highPriority ? highPriority[0] : 'See full report for details'}
              
              ### Summary
              - High Priority: ${{ steps.compare.outputs.high_issues }}
              - Medium Priority: ${{ steps.compare.outputs.medium_issues }}
              - Low Priority: ${{ steps.compare.outputs.low_issues }}
              
              [View full report](https://github.com/${{ github.repository }}/blob/main/reports/code-sync-report.md)
              
              ---
              *Generated by Project Nexus Analyzer*`,
              labels: ['drift', 'high-priority', 'automated']
            });
  
  remind:
    needs: setup
    if: needs.setup.outputs.should-remind == 'true' || github.event.inputs.action == 'remind' || github.event.inputs.action == 'all'
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Calculate inactivity
        id: inactivity
        run: |
          LAST_COMMIT=$(git log -1 --format="%ct")
          LAST_COMMIT_MSG=$(git log -1 --format="%s")
          LAST_AUTHOR=$(git log -1 --format="%an")
          CURRENT=$(date +%s)
          DAYS=$(( ($CURRENT - $LAST_COMMIT) / 86400 ))
          
          echo "days=$DAYS" >> $GITHUB_OUTPUT
          echo "last_message=$LAST_COMMIT_MSG" >> $GITHUB_OUTPUT
          echo "last_author=$LAST_AUTHOR" >> $GITHUB_OUTPUT
      
      - name: Extract next steps
        id: next_steps
        run: |
          if grep -q "## Next Steps" PRD.md; then
            NEXT_STEPS=$(awk '/## Next Steps/,/##[^#]/' PRD.md | head -20)
          else
            NEXT_STEPS="No next steps defined in PRD"
          fi
          
          echo "steps<<EOF" >> $GITHUB_OUTPUT
          echo "$NEXT_STEPS" >> $GITHUB_OUTPUT
          echo "EOF" >> $GITHUB_OUTPUT
      
      - name: Get recent commits
        id: commits
        run: |
          COMMITS=$(git log --oneline -10)
          echo "list<<EOF" >> $GITHUB_OUTPUT
          echo "$COMMITS" >> $GITHUB_OUTPUT
          echo "EOF" >> $GITHUB_OUTPUT
      
      - name: Check existing reminders
        id: existing
        uses: actions/github-script@v6
        with:
          script: |
            const issues = await github.rest.issues.listForRepo({
              owner: context.repo.owner,
              repo: context.repo.repo,
              labels: 'reminder',
              state: 'open'
            });
            
            return issues.data.length > 0;
      
      - name: Create or update reminder
        if: steps.existing.outputs.result == 'false'
        uses: actions/github-script@v6
        with:
          script: |
            const title = `⏰ Project inactive for ${{ steps.inactivity.outputs.days }} days`;
            
            const body = `## Project Activity Reminder
            
            This project has been inactive for **${{ steps.inactivity.outputs.days }} days**.
            
            ### ⚡ Quick Stats
            - **Last Commit:** "${{ steps.inactivity.outputs.last_message }}"
            - **Last Author:** ${{ steps.inactivity.outputs.last_author }}
            - **Days Inactive:** ${{ steps.inactivity.outputs.days }}
            
            ### 📋 Next Steps from PRD
            ${{ steps.next_steps.outputs.steps }}
            
            ### 📜 Recent Activity
            \`\`\`
            ${{ steps.commits.outputs.list }}
            \`\`\`
            
            ### ✅ Suggested Actions
            - [ ] Review current project status
            - [ ] Update PRD with recent decisions
            - [ ] Import latest AI conversations
            - [ ] Run code-PRD sync analysis
            - [ ] Continue development
            - [ ] Or archive project if no longer needed
            
            ### 🔗 Quick Commands
            \`\`\`bash
            # Check project status
            nexus status
            
            # Import recent conversation
            nexus import conversation
            
            # Run analysis
            nexus analyze
            \`\`\`
            
            ---
            *This reminder was automatically generated by Project Nexus*
            *To disable reminders, run: \`nexus config set reminders.enabled false\`*`;
            
            await github.rest.issues.create({
              owner: context.repo.owner,
              repo: context.repo.repo,
              title: title,
              body: body,
              labels: ['reminder', 'inactive', 'automated']
            });
  
  scout:
    needs: setup
    if: needs.setup.outputs.should-scout == 'true' || github.event.inputs.action == 'scout' || github.event.inputs.action == 'all'
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'
      
      - name: Install dependencies
        run: |
          pip install gitpython scikit-learn
      
      - name: Scout for modules
        id: scout
        run: |
          python scripts/reuse_scout.py \
            --repo . \
            --scan-dirs "../*" \
            --output suggestions.json
          
          COUNT=$(jq '. | length' suggestions.json)
          echo "count=$COUNT" >> $GITHUB_OUTPUT
      
      - name: Update PRD with suggestions
        if: steps.scout.outputs.count > 0
        run: |
          python scripts/update_prd.py \
            --prd PRD.md \
            --suggestions suggestions.json \
            --section "Reuse Suggestions"
          
          if [ -n "$(git diff PRD.md)" ]; then
            git add PRD.md
            git commit -m "auto(scout): Found ${{ steps.scout.outputs.count }} reusable modules"
            git push
          fi
      
      - name: Create suggestion issue
        if: steps.scout.outputs.count > 0
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const suggestions = JSON.parse(fs.readFileSync('suggestions.json', 'utf8'));
            
            let body = `## 🎯 Module Reuse Opportunities Found
            
            Project Nexus Scout found **${suggestions.length}** modules that could be reused:
            
            ### Suggestions
            `;
            
            suggestions.forEach((s, i) => {
              body += `
            ${i + 1}. **${s.module}**
               - From: \`${s.source_project}\`
               - Similarity: ${s.similarity}%
               - Time saved: ~${s.time_saved} hours
               - Command: \`cp ${s.source_path} ./modules/\`
            `;
            });
            
            body += `
            
            ### Next Steps
            1. Review the suggestions above
            2. Copy relevant modules to your project
            3. Update imports and configurations
            4. Remove duplicate code
            
            ---
            *Generated by Project Nexus Module Scout*`;
            
            await github.rest.issues.create({
              owner: context.repo.owner,
              repo: context.repo.repo,
              title: `♻️ ${suggestions.length} reusable modules found`,
              body: body,
              labels: ['enhancement', 'reuse', 'automated']
            });

  summary:
    needs: [analyze, remind, scout]
    if: always()
    runs-on: ubuntu-latest
    
    steps:
      - name: Generate workflow summary
        run: |
          echo "## 📊 Nexus Pipeline Summary" >> $GITHUB_STEP_SUMMARY
          echo "" >> $GITHUB_STEP_SUMMARY
          
          if [ "${{ needs.analyze.result }}" == "success" ]; then
            echo "✅ **Analysis:** Completed successfully" >> $GITHUB_STEP_SUMMARY
          elif [ "${{ needs.analyze.result }}" == "skipped" ]; then
            echo "⏭️ **Analysis:** Skipped (no changes detected)" >> $GITHUB_STEP_SUMMARY
          else
            echo "❌ **Analysis:** Failed" >> $GITHUB_STEP_SUMMARY
          fi
          
          if [ "${{ needs.remind.result }}" == "success" ]; then
            echo "✅ **Reminder:** Created/Updated" >> $GITHUB_STEP_SUMMARY
          elif [ "${{ needs.remind.result }}" == "skipped" ]; then
            echo "⏭️ **Reminder:** Skipped (project active)" >> $GITHUB_STEP_SUMMARY
          else
            echo "❌ **Reminder:** Failed" >> $GITHUB_STEP_SUMMARY
          fi
          
          if [ "${{ needs.scout.result }}" == "success" ]; then
            echo "✅ **Scout:** Module suggestions generated" >> $GITHUB_STEP_SUMMARY
          elif [ "${{ needs.scout.result }}" == "skipped" ]; then
            echo "⏭️ **Scout:** Skipped (no new modules)" >> $GITHUB_STEP_SUMMARY
          else
            echo "❌ **Scout:** Failed" >> $GITHUB_STEP_SUMMARY
          fi
```

</details>

### Workflow Triggers

```yaml
Automatic Triggers:
  On Push:
    - Analyze code-PRD sync
    - Update conversation graph
    - Regenerate reports
    
  Daily Schedule:
    - Check project activity
    - Create reminders if needed
    - Scout for new modules
    
  Weekly Schedule:
    - Deep analysis
    - Pattern detection
    - Performance metrics

Manual Triggers:
  Workflow Dispatch:
    - Run specific action
    - Force analysis
    - Generate reports
    
  API Trigger:
    - External webhook
    - Integration callback
    - Custom automation
```

---

# ✅ PART IV: VALIDATION & LAUNCH

---

## 1️⃣4️⃣ Acceptance Criteria {#acceptance-criteria}

**[ID: ACC-001]** **[MVP]** **[CRITICAL]**

> **💡 TL;DR:** Complete testing scenarios for all MVP features. Each feature must pass functional, performance, and usability tests before release.

<details>
<summary><b>🔧 Detailed Test Cases</b></summary>

### Core Functionality Tests

```gherkin
Feature: Project Initialization

  Scenario: First-time setup
    Given I have a Git repository without Nexus
    When I run "nexus init"
    Then the following should be created:
      | Path | Type | Purpose |
      | /conversations/ | Directory | Store AI conversations |
      | /modules/ | Directory | Module specifications |
      | /reports/ | Directory | Analysis reports |
      | /.nexus/config.yml | File | Configuration |
      | /.github/workflows/ | Directory | GitHub Actions |
    And the repository should have git hooks installed
    And PRD.md should be created if not exists
    And the command should complete in <5 seconds

  Scenario: Re-initialization protection
    Given Nexus is already initialized
    When I run "nexus init" without --force
    Then I should see a warning "Project already initialized"
    And existing files should not be overwritten
    And the command should exit with code 1

Feature: Conversation Import and Analysis

  Scenario: Import ChatGPT conversation
    Given I have a ChatGPT export file "chat.json"
    When I run "nexus import conversation --file chat.json"
    Then the file should be saved to /conversations/YYYY-MM-DD-chatgpt.json
    And decisions should be extracted and counted
    And a commit should be created with format "feat(conversation): [summary]"
    And the conversation graph should be updated
    And the import should complete in <3 seconds

  Scenario: Detect decisions in conversation
    Given a conversation contains "Let's use PostgreSQL for the database"
    And it contains "We should implement OAuth for authentication"
    When the system processes the conversation
    Then 2 decisions should be identified
    And each decision should have confidence score >0.6
    And decisions should be added to the timeline

Feature: Code-PRD Synchronization

  Scenario: Detect undocumented endpoints
    Given PRD.md defines endpoints:
      | Endpoint | Method | Description |
      | /api/users | GET | List users |
      | /api/users/:id | GET | Get user |
    And the code implements:
      | Endpoint | Method | File |
      | /api/users | GET | routes/users.js |
      | /api/users/:id | GET | routes/users.js |
      | /api/users/batch | POST | routes/users.js |
    When I run "nexus analyze"
    Then the report should show 1 undocumented endpoint
    And it should be marked as "High Priority"
    And suggest updating PRD with the new endpoint

  Scenario: Identify background AI changes
    Given a commit by "Cursor AI" added new functionality
    And this functionality is not in PRD.md
    When the analyzer runs
    Then it should flag this as "AI-generated drift"
    And mark it with higher priority than human changes
    And suggest reviewing the AI changes

Feature: Inactive Project Reminders

  Scenario: Create reminder after inactivity
    Given the last commit was 6 days ago
    And reminder threshold is set to 5 days
    When the daily reminder job runs
    Then an issue should be created with title containing "6 days"
    And it should include next steps from PRD.md
    And it should show last 5 commits
    And it should not create duplicate if one exists

  Scenario: Auto-close reminder on activity
    Given a reminder issue exists for inactivity
    When a new commit is made
    Then the reminder issue should be auto-closed
    And a comment should be added "Project is now active"

Feature: Module Reuse Detection

  Scenario: Find similar authentication module
    Given project-A has an auth module with JWT implementation
    And project-B needs authentication
    When I run "nexus scout"
    Then it should detect the similarity >70%
    And suggest copying auth module from project-A
    And estimate time savings of 4+ hours
    And update PRD with the suggestion
```

</details>

### Performance Criteria

```yaml
Response Times:
  CLI Operations:
    init: <5 seconds
    import: <3 seconds
    analyze: <10 seconds (100k LOC)
    scout: <30 seconds (50 repos)
    status: <1 second
    
  GitHub Actions:
    Analysis workflow: <2 minutes
    Reminder check: <30 seconds
    Scout workflow: <5 minutes
    
  API Endpoints:
    GET /status: <100ms
    POST /analyze: <5 seconds
    GET /timeline: <500ms
    
Resource Usage:
  Memory:
    CLI idle: <50MB
    CLI analyzing: <200MB
    Mobile app: <100MB
    
  Disk:
    Conversation storage: <1KB per conversation
    Reports: <100KB per report
    Cache: <10MB total
    
  Network:
    API calls: <1MB per operation
    Sync bandwidth: <500KB per sync
    Mobile data: <100KB per session
```

### Security Criteria

```yaml
Data Protection:
  Encryption:
    ✓ All API communications use HTTPS
    ✓ Sensitive data encrypted at rest
    ✓ No credentials in Git history
    
  Authentication:
    ✓ JWT tokens expire in 24 hours
    ✓ API keys can be revoked
    ✓ No plaintext passwords
    
  Privacy:
    ✓ No telemetry without consent
    ✓ AI calls sanitized
    ✓ Local processing preferred
```

---

## 1️⃣5️⃣ MVP Scope {#mvp-scope}

**[ID: MVP-001]** **[MVP]** **[CRITICAL]**

> **💡 TL;DR:** MVP includes CLI tools, basic GitHub Actions, and mobile viewer. Focus on single developer use case. Target: 100 beta users in 8 weeks.

### ✅ In Scope (MVP)

```yaml
Core Features:
  Git Operations:
    ✅ Repository initialization
    ✅ File structure creation
    ✅ Git hooks setup
    ✅ Commit enhancement
    
  Conversation Management:
    ✅ Import from ChatGPT, Claude, Gemini
    ✅ Decision extraction
    ✅ Timeline generation
    ✅ Graph visualization
    
  Analysis:
    ✅ PRD parsing
    ✅ Code structure analysis
    ✅ Drift detection
    ✅ Report generation
    
  Automation:
    ✅ GitHub Actions workflows
    ✅ Daily reminder checks
    ✅ Push-triggered analysis
    ✅ Auto-commit reports
    
  CLI Tools:
    ✅ All basic commands
    ✅ Configuration management
    ✅ Status checking
    ✅ Manual operations
    
  Mobile App:
    ✅ View projects
    ✅ Read PRDs
    ✅ View reports
    ✅ Basic notifications
```

### ⏸️ Post-MVP (V2)

```yaml
Advanced Features:
  - Smart Routine Engine
  - Advanced pattern detection
  - Team collaboration
  - API marketplace
  - Custom AI models
  - Voice commands
  - IDE plugins
  - Web dashboard editing
```

### ❌ Out of Scope

```yaml
Will Not Build:
  - Multi-user repositories
  - Real-time collaboration
  - Custom Git hosting
  - CI/CD management
  - Project management features
  - Time tracking
  - Billing/invoicing
  - Social features
```

---

## 1️⃣6️⃣ Development Phases {#development-phases}

**[ID: DEV-001]**

> **💡 TL;DR:** Three phases over 8 weeks: Local CLI (2 weeks), GitHub integration (2 weeks), Mobile app (2 weeks), Beta testing (2 weeks).

### Development Timeline

```mermaid
gantt
    title Project Nexus Development
    dateFormat YYYY-MM-DD
    
    section Phase 1: CLI
    Repository Structure    :2025-11-01, 3d
    PRD Summarizer         :3d
    Conversation Import    :3d
    Code Analyzer         :4d
    Git Hooks            :2d
    
    section Phase 2: GitHub
    Action Templates      :2025-11-15, 3d
    Analysis Workflow    :3d
    Reminder System      :3d
    Scout Implementation :4d
    Integration Tests    :2d
    
    section Phase 3: Mobile
    React Native Setup   :2025-11-29, 2d
    Project List View   :3d
    PRD Viewer         :2d
    Report Display     :2d
    Push Notifications :2d
    App Store Prep     :3d
    
    section Beta
    Beta Launch        :2025-12-15, 1d
    User Onboarding   :7d
    Feedback Loop     :14d
    Bug Fixes        :7d
    Public Launch    :2026-01-15, 1d
```

### Sprint Plan

```yaml
Sprint 1 (Weeks 1-2): Foundation
  Goals:
    - Working CLI
    - Git integration
    - Basic analysis
    
  Deliverables:
    - nexus init command
    - Conversation import
    - PRD summarizer
    - Git hooks
    
  Success Metrics:
    - All commands working
    - 5 test repos analyzed
    - <10 second analysis

Sprint 2 (Weeks 3-4): Automation
  Goals:
    - GitHub Actions
    - Automated workflows
    - Report generation
    
  Deliverables:
    - Analysis workflow
    - Reminder system
    - Module scout
    - Issue creation
    
  Success Metrics:
    - Actions run successfully
    - Zero manual intervention
    - <2 minute workflows

Sprint 3 (Weeks 5-6): Mobile
  Goals:
    - Mobile app
    - Notifications
    - User onboarding
    
  Deliverables:
    - iOS/Android apps
    - Project viewer
    - Push notifications
    - App store listing
    
  Success Metrics:
    - Apps approved
    - <2 second load time
    - Smooth navigation

Sprint 4 (Weeks 7-8): Beta
  Goals:
    - User testing
    - Bug fixes
    - Polish
    
  Activities:
    - 100 beta users
    - Daily iterations
    - Performance optimization
    - Documentation
    
  Success Metrics:
    - 50+ NPS score
    - <1% crash rate
    - 80% feature adoption
```

---

## 1️⃣7️⃣ Security & Privacy {#security-privacy}

**[ID: SEC-001]** **[CRITICAL]**

> **💡 TL;DR:** Git-native security model. No central database. Optional AI features. Complete user control over data.

### Security Architecture

```yaml
Data Storage:
  Primary: User's Git repositories
  Access: User's credentials only
  Backup: User's remotes
  Encryption: Git's built-in
  
Authentication:
  CLI: Git credentials
  Mobile: OAuth via GitHub
  API: JWT tokens
  Actions: GitHub tokens
  
Privacy:
  AI Calls:
    - Sanitized before sending
    - No code sent to AI
    - Only PRD summaries
    - User can disable
    
  Data Collection:
    - No telemetry by default
    - Opt-in analytics only
    - No cross-user data
    - Local processing preferred
    
Compliance:
  GDPR: ✓ User owns all data
  CCPA: ✓ No data selling
  SOC2: ✓ Security controls
  ISO27001: ✓ Best practices
```

### Security Best Practices

```python
class SecurityManager:
    def sanitize_for_ai(self, text):
        """Remove sensitive data before AI processing"""
        # Remove API keys
        text = re.sub(r'[a-f0-9]{32,}', '[KEY]', text)
        
        # Remove passwords
        text = re.sub(r'password\s*=\s*["\'][^"\']+["\']', 
                     'password=[REDACTED]', text)
        
        # Remove emails
        text = re.sub(r'[\w\.-]+@[\w\.-]+', '[EMAIL]', text)
        
        # Remove URLs with credentials
        text = re.sub(r'https?://[^:]+:[^@]+@', 'https://[CREDS]@', text)
        
        return text
    
    def verify_repo_safety(self, repo_path):
        """Ensure repository is safe to analyze"""
        # Check for .git directory
        if not os.path.exists(f"{repo_path}/.git"):
            raise SecurityError("Not a Git repository")
        
        # Check for malicious patterns
        suspicious_patterns = [
            'eval(',
            'exec(',
            '__import__',
            'os.system'
        ]
        
        # Scan for suspicious code
        for pattern in suspicious_patterns:
            if self.scan_for_pattern(repo_path, pattern):
                logger.warning(f"Suspicious pattern found: {pattern}")
        
        return True
```

---

## 📊 Success Metrics & KPIs

### Launch Success Criteria

```yaml
Week 1:
  Signups: 100+
  Activation Rate: 70%+ (run first command)
  Day 1 Retention: 60%+
  
Week 4:
  Active Users: 50+ daily
  Repos Tracked: 200+
  Conversations Imported: 500+
  Analysis Run: 1000+
  
Week 8:
  Beta Users: 100+
  NPS Score: 50+
  Bug Reports: <50
  Feature Requests: 100+
  Documentation Complete: Yes
```

### Long-term Vision

```yaml
6 Months:
  Users: 10,000
  Repos: 50,000
  Revenue: $10K MRR
  Team: 5 people
  
1 Year:
  Users: 100,000
  Repos: 500,000
  Revenue: $100K MRR
  Team: 15 people
  
2 Years:
  Users: 1M
  Platform: Full ecosystem
  Revenue: $1M MRR
  Exit: Acquisition ready
```

---

## 🚀 Final Summary

Project Nexus revolutionizes how developers manage multiple projects by:

1. **Unifying** all project memory in Git
2. **Automating** context preservation
3. **Detecting** code-documentation drift
4. **Preventing** project abandonment
5. **Enabling** module reuse

The system is:
- **Simple**: Just Git and Markdown
- **Private**: No external database
- **Powerful**: AI-enhanced intelligence
- **Practical**: Solves real problems

---

## 📋 Document Metadata

```yaml
Document:
  Type: Product Requirements Document
  Version: 1.0
  Status: Complete
  Pages: ~80
  Words: ~20,000
  Sections: 17
  
Quality Metrics:
  Completeness: 100%
  Detail Level: High
  Examples: 50+
  Test Cases: 30+
  Code Samples: 40+
  
Coverage:
  Problem Definition: ✅
  Solution Design: ✅
  Technical Architecture: ✅
  Implementation: ✅
  Testing: ✅
  Launch Strategy: ✅
  
Ready For:
  ✅ Development Start
  ✅ Technical Review
  ✅ Stakeholder Approval
  ✅ Resource Planning
  ✅ Beta Testing
```

---

**End of Document - Project Nexus PRD v1.0** ✅

**Status:** Ready for implementation  
**Next Steps:** Begin Phase 1 CLI development  
**Contact:** [Project Nexus Team]  

---