"""Type stubs for the Rust-backed ``m2pa._m2pa`` extension module."""

from __future__ import annotations

# ── Protocol constants (RFC 4165 §2) ─────────────────────────────────────────
VERSION: int
MESSAGE_CLASS: int
MESSAGE_TYPE_USER_DATA: int
MESSAGE_TYPE_LINK_STATUS: int
SCTP_PPID: int

class M2paError(Exception):
    """M2PA protocol / codec error (RFC 4165)."""

class LinkState:
    """Link Status states (RFC 4165 §3.3).

    A PyO3 enum: members compare equal to their on-wire integer (``int(...)``
    yields the wire value), but it is not a Python ``enum.IntEnum`` (no
    iteration, no ``.value``).
    """

    Alignment: LinkState
    ProvingNormal: LinkState
    ProvingEmergency: LinkState
    Ready: LinkState
    ProcessorOutage: LinkState
    ProcessorRecovered: LinkState
    Busy: LinkState
    BusyEnded: LinkState
    def __int__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class M2paState:
    """The local link state tracked by :class:`StateMachine` (RFC 4165 §4)."""

    OutOfService: M2paState
    NotAligned: M2paState
    Aligned: M2paState
    Proving: M2paState
    AlignedReady: M2paState
    InService: M2paState
    def __int__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class LinkStatus:
    """An M2PA Link Status message (SCTP stream 0)."""

    bsn: int
    fsn: int
    state: LinkState
    def __init__(
        self, state: LinkState, *, bsn: int = 0xFFFFFF, fsn: int = 0xFFFFFF
    ) -> None: ...
    def encode(self) -> bytes:
        """Encode the complete M2PA message (20 bytes)."""

class UserData:
    """An M2PA User Data message (SCTP stream 1), carrying an MTP3 MSU."""

    bsn: int
    fsn: int
    priority: int
    msu: bytes
    def __init__(
        self, msu: bytes, *, priority: int = 0, bsn: int = 0, fsn: int = 0
    ) -> None: ...
    def encode(self) -> bytes:
        """Encode the complete M2PA message (common header + M2PA header + body)."""

class StateMachine:
    """The RFC 4165 §4 link state machine."""

    def __init__(self) -> None: ...
    @property
    def state(self) -> M2paState: ...
    def start(self) -> bool:
        """Begin alignment from ``OutOfService``; returns ``True`` if valid."""
    def on_link_status(self, link_state: LinkState) -> M2paState:
        """Feed a received Link Status; returns the resulting local state."""
    def stop(self) -> None:
        """Force the link out of service."""

def decode(data: bytes) -> LinkStatus | UserData:
    """Decode a complete M2PA message into a :class:`LinkStatus` or :class:`UserData`."""
