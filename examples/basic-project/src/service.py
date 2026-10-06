"""Example service (fictitious)."""


def get_user(user_id: str) -> dict:
    """Return a minimal user record."""
    return {"id": user_id, "email": "dev@example.com"}
