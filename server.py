import os
from pathlib import Path
from datetime import datetime
from typing import Optional, List, Dict, Any

import httpx
from fastapi import FastAPI, Request, HTTPException, UploadFile, File, Form
from fastapi.responses import HTMLResponse, JSONResponse, PlainTextResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates

import git
from nexus import Nexus

OPENROUTER_API_KEY = os.getenv("OPENROUTER_API_KEY", "")
OPENROUTER_BASE_URL = os.getenv("OPENROUTER_BASE_URL", "https://openrouter.ai/api/v1")
DEFAULT_MODELS = [
    "openrouter/auto",
    "anthropic/claude-3.5-sonnet",
    "openai/gpt-4o-mini",
    "google/gemini-1.5-flash",
]

app = FastAPI(title="Nexus API")

# Static and templates
BASE_DIR = Path(__file__).parent
TEMPLATES_DIR = BASE_DIR / "web" / "templates"
STATIC_DIR = BASE_DIR / "web" / "static"

app.mount("/static", StaticFiles(directory=str(STATIC_DIR)), name="static")
templates = Jinja2Templates(directory=str(TEMPLATES_DIR))


def get_repo_context(repo_path: str = ".") -> Dict[str, Any]:
    repo = git.Repo(repo_path)
    prd_path = Path(repo_path) / "PRD.md"
    prd = prd_path.read_text() if prd_path.exists() else ""

    commits: List[Dict[str, str]] = []
    for c in list(repo.iter_commits("HEAD", max_count=10)):
        commits.append({
            "sha": c.hexsha[:7],
            "message": c.message.split("\n")[0],
            "author": c.author.name,
            "date": datetime.fromtimestamp(c.committed_date).strftime("%Y-%m-%d %H:%M"),
        })

    return {"prd": prd, "commits": commits}


@app.get("/", response_class=HTMLResponse)
async def dashboard(request: Request):
    return templates.TemplateResponse("index.html", {"request": request, "models": DEFAULT_MODELS})


@app.get("/api/status")
async def status():
    try:
        repo = git.Repo(".")
        last_commit = repo.head.commit
        days_ago = (datetime.now() - datetime.fromtimestamp(last_commit.committed_date)).days
        conv_dir = BASE_DIR / "conversations"
        conv_count = len(list((conv_dir).glob("*.json"))) if conv_dir.exists() else 0
        drift_path = BASE_DIR / "reports" / "drift.md"
        drift_exists = drift_path.exists()
        recent = []
        for c in list(repo.iter_commits("HEAD", max_count=5)):
            recent.append({
                "sha": c.hexsha[:7],
                "message": c.message.split("\n")[0],
                "author": c.author.name,
                "date": datetime.fromtimestamp(c.committed_date).strftime("%Y-%m-%d %H:%M"),
            })
        return {
            "last_commit": last_commit.message.split("\n")[0],
            "days_since_last_commit": days_ago,
            "conversations": conv_count,
            "drift_report": drift_exists,
            "recent_commits": recent,
        }
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/api/models")
async def list_models():
    # OpenRouter has a models endpoint, but for simplicity we return defaults
    return {"models": DEFAULT_MODELS}


@app.post("/api/chat")
async def chat(payload: Dict[str, Any]):
    if not OPENROUTER_API_KEY:
        raise HTTPException(status_code=400, detail="Missing OPENROUTER_API_KEY env var")

    message: str = payload.get("message", "").strip()
    model: str = payload.get("model") or DEFAULT_MODELS[0]
    repo_path: str = payload.get("repo_path", ".")

    if not message:
        raise HTTPException(status_code=400, detail="message is required")

    context = get_repo_context(repo_path)

    system_prompt = (
        "You are an expert code assistant. Use the PRD and recent commits to answer questions about this repo. "
        "If the answer is not in the context, say so and suggest next steps."
    )

    context_block = (
        f"PRD.md (truncated to 4000 chars):\n{context['prd'][:4000]}\n\n"  # keep lightweight
        f"Recent commits:\n" + "\n".join([f"- {c['date']} {c['sha']} {c['author']}: {c['message']}" for c in context["commits"]])
    )

    body = {
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": f"Context:\n{context_block}\n\nUser question: {message}"},
        ],
        "temperature": 0.3,
    }

    headers = {
        "Authorization": f"Bearer {OPENROUTER_API_KEY}",
        "HTTP-Referer": os.getenv("OPENROUTER_HTTP_REFERER", "http://localhost"),
        "X-Title": os.getenv("OPENROUTER_APP_NAME", "Nexus"),
    }

    url = f"{OPENROUTER_BASE_URL}/chat/completions"
    try:
        async with httpx.AsyncClient(timeout=60) as client:
            r = await client.post(url, json=body, headers=headers)
            if r.status_code >= 400:
                raise HTTPException(status_code=r.status_code, detail=r.text)
            data = r.json()
    except httpx.RequestError as e:
        raise HTTPException(status_code=502, detail=str(e))

    content = (
        data.get("choices", [{}])[0]
        .get("message", {})
        .get("content", "")
    )

    return {"answer": content}


@app.post("/api/analyze")
async def run_analyze():
    try:
        Nexus().analyze()
        drift_path = BASE_DIR / "reports" / "drift.md"
        text = drift_path.read_text() if drift_path.exists() else ""
        return {"ok": True, "report_path": "reports/drift.md", "report": text}
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/api/reports/drift", response_class=PlainTextResponse)
async def get_drift_report():
    path = BASE_DIR / "reports" / "drift.md"
    if not path.exists():
        raise HTTPException(status_code=404, detail="No drift report")
    return path.read_text()


@app.get("/api/timeline", response_class=PlainTextResponse)
async def get_timeline():
    try:
        Nexus().update_timeline()
        path = BASE_DIR / "conversations" / "index.md"
        return path.read_text() if path.exists() else "# Conversation Timeline\n\n(no events)"
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/api/conversations")
async def list_conversations():
    conv_dir = BASE_DIR / "conversations"
    items: list[dict[str, Any]] = []
    if conv_dir.exists():
        for p in sorted(conv_dir.glob("*.json"), key=lambda x: x.name, reverse=True)[:50]:
            items.append({"name": p.name, "size": p.stat().st_size})
    return {"count": len(items), "items": items}


@app.post("/api/conversations/import")
async def import_conversation(file: UploadFile = File(...), platform: str | None = Form(default=None)):
    try:
        # Save to a temp path and pass to Nexus importer
        tmp_path = BASE_DIR / "conversations" / (file.filename or "conversation.json")
        os.makedirs(tmp_path.parent, exist_ok=True)
        content = await file.read()
        with open(tmp_path, "wb") as f:
            f.write(content)
        Nexus().import_conversation(str(tmp_path), platform)
        return {"ok": True, "stored_as": str(tmp_path.name)}
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))
