"""m2pa — Rust-backed M2PA (RFC 4165) codec + link state machine for Python.

M2PA (MTP2 Peer-to-Peer Adaptation Layer) carries MTP3 signalling over SCTP, so
an SS7 linkset can ride IP the way it would ride TDM links. This package exposes
the same codec and state machine the Rust crate (``cargo add m2pa``) ships, from
one source tree / one version.

The wire work (header pack/unpack, body copy, the RFC 4165 §4 state transition
table) runs in Rust; Python just builds and inspects messages.
"""

from __future__ import annotations

from importlib.metadata import PackageNotFoundError, version

from ._m2pa import (
    MESSAGE_CLASS,
    MESSAGE_TYPE_LINK_STATUS,
    MESSAGE_TYPE_USER_DATA,
    SCTP_PPID,
    VERSION,
    LinkState,
    LinkStatus,
    M2paError,
    M2paState,
    StateMachine,
    UserData,
    decode,
)

try:
    __version__ = version("m2pa")
except PackageNotFoundError:  # running from a source checkout without an installed dist
    __version__ = "0.0.0+unknown"

__all__ = [
    # messages + codec
    "LinkStatus",
    "UserData",
    "decode",
    "M2paError",
    # enums
    "LinkState",
    "M2paState",
    # link state machine
    "StateMachine",
    # protocol constants
    "VERSION",
    "MESSAGE_CLASS",
    "MESSAGE_TYPE_USER_DATA",
    "MESSAGE_TYPE_LINK_STATUS",
    "SCTP_PPID",
    "__version__",
]
