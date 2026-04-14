from fastapi import FastAPI
from routes import games

app = FastAPI()

app.include_router(games.router)

@app.get("/up")
def hello():
    return {"message": "up"}