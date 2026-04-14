from celery.result import AsyncResult
from fastapi import APIRouter
from pydantic import BaseModel

from celery_app import celery_app
from services.tasks import compute_ai_move_task

router = APIRouter(prefix="/games")


class MoveRequest(BaseModel):
    board: list
    current_player: int


@router.post("/ai-move")
def ai_move(req: MoveRequest):
    task = compute_ai_move_task.delay(req.model_dump())
    return {"task_id": task.id}


@router.get("/ai-move/{task_id}")
def ai_move_result(task_id: str):
    result = AsyncResult(task_id, app=celery_app)
    if result.state == "PENDING":
        return {"status": "pending"}
    if result.state == "STARTED":
        return {"status": "started"}
    if result.state == "FAILURE":
        return {"status": "failed", "error": str(result.result)}
    return {"status": "completed", "result": result.result}
