import os
from fastapi import FastAPI, Request, Query, HTTPException, Body
from fastapi.responses import StreamingResponse, JSONResponse, FileResponse
from fastapi.staticfiles import StaticFiles
from typing import Optional, Dict, Any
import threading
import json
import asyncio

from . import config
from .monitor import Monitor
from . import services
from fastapi.middleware.trustedhost import TrustedHostMiddleware

app = FastAPI()
app.add_middleware(TrustedHostMiddleware, allowed_hosts=["127.0.0.1", "localhost", "::1"])

m = Monitor()
threading.Thread(target=m.update_quota, daemon=True).start()

@app.get("/api/status")
def api_status():
    return m.full()

@app.get("/api/live/stream")
async def api_live_stream(request: Request):
    async def event_generator():
        while True:
            if await request.is_disconnected():
                break
            live = m.streamer.get_live()
            show = (m.service()["state"] == "running" or 
                    (live is not None and live.get("is_syncing")))
            payload = json.dumps({"live": live if show else None})
            yield f"data: {payload}\n\n"
            await asyncio.sleep(1 if show else 3)
    return StreamingResponse(event_generator(), media_type="text/event-stream", headers={"Cache-Control": "no-cache", "X-Accel-Buffering": "no"})

@app.post("/api/trigger")
def api_trigger():
    return services.api_trigger(m)

@app.post("/api/cancel")
def api_cancel():
    return services.api_cancel(m)

@app.get("/api/bwlimit")
def api_bwlimit():
    return services.api_bwlimit()

@app.post("/api/bwlimit_save")
def api_bwlimit_save(limit: str = ''):
    return services.api_bwlimit_save(limit)

@app.get("/api/dryrun")
def api_dryrun():
    return services.api_dryrun()

@app.get("/api/tree")
def api_tree(dir: str = ''):
    return services.api_tree(dir)

@app.get("/api/search")
def api_search(dir: str = '', q: str = ''):
    return services.api_search(dir, q)

@app.get("/api/filters")
def api_filters():
    return services.api_filters()

@app.post("/api/filters_add")
def api_filters_add(rule: str = ''):
    return services.api_filters_add(rule)

@app.get("/api/open")
def api_open(path: str = '', dir_only: str = '0'):
    return services.api_open(path, dir_only)

@app.get("/api/delete_preview")
def api_delete_preview(path: str = ''):
    return services.api_delete_preview(path)

@app.get("/api/drive_check")
def api_drive_check(path: str = ''):
    return services.api_drive_check(path)

@app.post("/api/filters_remove")
def api_filters_remove(rule: str = ''):
    return services.api_filters_remove(rule)

@app.get("/api/match_rules")
def api_match_rules(path: str = ''):
    return services.api_match_rules(path)

@app.get("/api/rule_impact")
def api_rule_impact(rule: str = ''):
    return services.api_rule_impact(rule)

@app.post("/api/filters_save")
def api_filters_save(data: Dict[str, Any] = Body(...)):
    return services.api_filters_save(data)

@app.post("/api/delete")
def api_delete(data: Dict[str, Any] = Body(...)):
    return services.api_delete(data)

@app.post("/api/drive_delete")
def api_drive_delete(data: Dict[str, Any] = Body(...)):
    return services.api_drive_delete(data)

@app.post("/api/rule_delete")
def api_rule_delete(data: Dict[str, Any] = Body(...)):
    return services.api_rule_delete(data)


static_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))

for d in ["css", "js", "fonts"]:
    dpath = os.path.join(static_dir, d)
    if os.path.exists(dpath):
        app.mount(f"/{d}", StaticFiles(directory=dpath), name=d)

@app.get("/")
@app.get("/index.html")
def index():
    return FileResponse(os.path.join(static_dir, "index.html"))

@app.get("/style.css")
def style_css():
    return FileResponse(os.path.join(static_dir, "style.css"))

@app.get("/app.js")
def app_js():
    return FileResponse(os.path.join(static_dir, "app.js"))

def main():
    import uvicorn
    uvicorn.run("backend.server:app", host="127.0.0.1", port=config.PORT, reload=False)

if __name__ == "__main__":
    main()
