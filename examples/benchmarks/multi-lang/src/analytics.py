"""Simple analytics module for tracking events."""

from dataclasses import dataclass
from datetime import datetime
from typing import Optional

@dataclass
class Event:
    """A single analytics event."""
    name: str
    timestamp: datetime
    properties: dict
    user_id: Optional[str] = None


class EventStore:
    """In-memory event store with basic query support."""

    def __init__(self):
        self._events: list[Event] = []

    def track(self, name: str, properties: dict = None, user_id: str = None) -> Event:
        """Record a new event."""
        event = Event(
            name=name,
            timestamp=datetime.now(),
            properties=properties or {},
            user_id=user_id,
        )
        self._events.append(event)
        return event

    def count(self, name: str) -> int:
        """Count events by name."""
        return sum(1 for e in self._events if e.name == name)

    def by_user(self, user_id: str) -> list[Event]:
        """Get all events for a user."""
        return [e for e in self._events if e.user_id == user_id]

    def recent(self, n: int = 10) -> list[Event]:
        """Get the N most recent events."""
        return self._events[-n:]


MAX_EVENTS = 10_000
