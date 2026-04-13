from fastapi import APIRouter
from pydantic import BaseModel
from services.rust_bridge import compute_ai_move

router = APIRouter(prefix="/games")

class MoveRequest(BaseModel):
    board: list
    current_player: int

@router.post("/ai-move")
def ai_move(req: MoveRequest):
    result = compute_ai_move(req.dict())
    return result

@router.get("/hello")
def hello():
    return {"message": "hello"}