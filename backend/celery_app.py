import os

from celery import Celery

REDIS_URL = os.getenv("GOMOKU_REDIS_URL", "redis://redis:6379/0")

celery_app = Celery(
    "gomoku",
    broker=REDIS_URL,
    backend=REDIS_URL,
    include=["services.tasks"],
)

celery_app.conf.update(
    task_serializer="json",
    result_serializer="json",
    accept_content=["json"],
    task_track_started=True,
    result_expires=3600,
)
