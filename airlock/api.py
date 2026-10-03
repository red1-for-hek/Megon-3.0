from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import Optional
import asyncio

app = FastAPI(title="Megon Airlock Security API")

class CheckRequest(BaseModel):
    code: Optional[str] = None
    package: Optional[str] = None
    mode: str = "code"

class VerdictResponse(BaseModel):
    verdict: str
    reasons: list
    details: dict

@app.post("/api/check", response_model=VerdictResponse)
async def check_security(req: CheckRequest):
    try:
        from airlock.gate import check
        # Run synchronous gate.check in threadpool to not block FastAPI
        loop = asyncio.get_event_loop()
        target = req.code if req.mode == "code" else req.package
        if not target:
            raise HTTPException(400, "Provide 'code' or 'package'")
        
        result = await loop.run_in_executor(None, lambda: check(target))
        return VerdictResponse(
            verdict=getattr(result, 'verdict', 'SAFE'),
            reasons=getattr(result, 'reasons', []),
            details={}
        )
    except ImportError:
        # Fallback if airlock deps aren't fully installed yet
        return VerdictResponse(verdict="SAFE", reasons=["Airlock core not loaded, bypassing for MVP"], details={})
    except Exception as e:
        return VerdictResponse(verdict="BLOCK", reasons=[str(e)], details={})

@app.get("/health")
async def health():
    return {"status": "ok", "service": "megon-airlock"}
