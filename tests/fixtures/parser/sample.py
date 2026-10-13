import os
import sys as system
from pathlib import Path
from . import local
from .helpers import run as go

CONSTANT_LIMIT = 100

def load(path):
    with open(path) as handle:
        return handle.read()


class Store:
    def __init__(self, root):
        self.root = root

    def get(self, key):
        return key

    @property
    def size(self):
        return 0


@dataclass
class Point:
    x: int
    y: int


@app.route("/ping")
def ping():
    return "pong"
