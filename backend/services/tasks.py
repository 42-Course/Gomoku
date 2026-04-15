import random
import time
from celery_app import celery

@celery.task(name="compute_ai_move")
def compute_ai_move_task(state: dict) -> dict:
    """
    Calls the Rust engine to compute the next move.

    TODO:
    - Implement subprocess call to Rust binary
    - Define JSON contract
    """
    time.sleep(random.uniform(1, 5))
    raise NotImplementedError("Rust bridge not implemented yet")
