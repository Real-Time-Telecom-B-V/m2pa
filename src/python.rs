//! PyO3 bindings — `pip install m2pa` gives a Rust-backed wheel exposing the
//! **same** M2PA (RFC 4165) codec and link state machine the crate ships.
//!
//! Compiled only with `--features python`; the default crate build is pyo3-free, so
//! `cargo add m2pa` / crates.io consumers pull zero pyo3. Two entry points share one
//! `add_contents()`:
//! * `#[pymodule] fn _m2pa` — the standalone wheel (maturin `module-name`).
//! * `pub fn register(py, parent)` — mount `m2pa` as a submodule of another
//!   extension, so a host can expose m2pa without a second shared object.
//!
//! The Python surface is a faithful mirror of the Rust one: `LinkStatus` /
//! `UserData` build and parse full M2PA messages, `decode()` dispatches on the
//! message type, and `StateMachine` drives the RFC 4165 §4 link state machine.

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule};

use crate::{
    LinkState as CoreLinkState, LinkStatusMessage, M2paError as CoreM2paError, M2paMessage,
    M2paState as CoreM2paState, M2paStateMachine, UserDataMessage, MESSAGE_CLASS_M2PA,
    MESSAGE_TYPE_LINK_STATUS, MESSAGE_TYPE_USER_DATA, SCTP_PPID, VERSION,
};

// ── Error mapping ───────────────────────────────────────────────────────────
create_exception!(
    m2pa,
    M2paError,
    PyException,
    "M2PA protocol / codec error (RFC 4165)."
);

fn m2pa_err(e: CoreM2paError) -> PyErr {
    M2paError::new_err(e.to_string())
}

// ── LinkState (RFC 4165 §3.3) ───────────────────────────────────────────────
/// Link Status states carried in a Link Status message. Integer values are the
/// on-wire encoding (`LinkState.Ready == 4`).
#[pyclass(name = "LinkState", module = "m2pa._m2pa", eq, eq_int, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyLinkState {
    Alignment = 1,
    ProvingNormal = 2,
    ProvingEmergency = 3,
    Ready = 4,
    ProcessorOutage = 5,
    ProcessorRecovered = 6,
    Busy = 7,
    BusyEnded = 8,
}

impl PyLinkState {
    fn to_core(self) -> CoreLinkState {
        match self {
            PyLinkState::Alignment => CoreLinkState::Alignment,
            PyLinkState::ProvingNormal => CoreLinkState::ProvingNormal,
            PyLinkState::ProvingEmergency => CoreLinkState::ProvingEmergency,
            PyLinkState::Ready => CoreLinkState::Ready,
            PyLinkState::ProcessorOutage => CoreLinkState::ProcessorOutage,
            PyLinkState::ProcessorRecovered => CoreLinkState::ProcessorRecovered,
            PyLinkState::Busy => CoreLinkState::Busy,
            PyLinkState::BusyEnded => CoreLinkState::BusyEnded,
        }
    }

    fn from_core(s: CoreLinkState) -> Self {
        match s {
            CoreLinkState::Alignment => PyLinkState::Alignment,
            CoreLinkState::ProvingNormal => PyLinkState::ProvingNormal,
            CoreLinkState::ProvingEmergency => PyLinkState::ProvingEmergency,
            CoreLinkState::Ready => PyLinkState::Ready,
            CoreLinkState::ProcessorOutage => PyLinkState::ProcessorOutage,
            CoreLinkState::ProcessorRecovered => PyLinkState::ProcessorRecovered,
            CoreLinkState::Busy => PyLinkState::Busy,
            CoreLinkState::BusyEnded => PyLinkState::BusyEnded,
        }
    }
}

// ── M2paState (RFC 4165 §4 link state machine) ──────────────────────────────
/// The link's local state as tracked by [`StateMachine`].
#[pyclass(
    name = "M2paState",
    module = "m2pa._m2pa",
    eq,
    eq_int,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyM2paState {
    OutOfService,
    NotAligned,
    Aligned,
    Proving,
    AlignedReady,
    InService,
}

impl PyM2paState {
    fn from_core(s: CoreM2paState) -> Self {
        match s {
            CoreM2paState::OutOfService => PyM2paState::OutOfService,
            CoreM2paState::NotAligned => PyM2paState::NotAligned,
            CoreM2paState::Aligned => PyM2paState::Aligned,
            CoreM2paState::Proving => PyM2paState::Proving,
            CoreM2paState::AlignedReady => PyM2paState::AlignedReady,
            CoreM2paState::InService => PyM2paState::InService,
        }
    }
}

// ── LinkStatus message ──────────────────────────────────────────────────────
/// An M2PA Link Status message (sent on SCTP stream 0). `encode()` produces the
/// full 20-byte message; `m2pa.decode(...)` returns one of these.
#[pyclass(name = "LinkStatus", module = "m2pa._m2pa", skip_from_py_object)]
#[derive(Clone)]
pub struct PyLinkStatus {
    #[pyo3(get)]
    pub bsn: u32,
    #[pyo3(get)]
    pub fsn: u32,
    #[pyo3(get)]
    pub state: PyLinkState,
}

#[pymethods]
impl PyLinkStatus {
    /// Link Status messages carry no user data; BSN/FSN default to the RFC's
    /// initial `0xFFFFFF` sentinel used before the link is in service.
    #[new]
    #[pyo3(signature = (state, *, bsn = 0x00FF_FFFF, fsn = 0x00FF_FFFF))]
    fn new(state: PyLinkState, bsn: u32, fsn: u32) -> Self {
        Self { bsn, fsn, state }
    }

    /// Encode the complete M2PA message (common header + M2PA header + body).
    fn encode<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let msg = M2paMessage::LinkStatus {
            bsn: self.bsn,
            fsn: self.fsn,
            message: LinkStatusMessage::new(self.state.to_core()),
        };
        let bytes = msg.encode().map_err(m2pa_err)?;
        Ok(PyBytes::new(py, &bytes))
    }

    fn __repr__(&self) -> String {
        format!(
            "LinkStatus(state={}, bsn={}, fsn={})",
            self.state.to_core(),
            self.bsn,
            self.fsn
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.bsn == other.bsn && self.fsn == other.fsn && self.state == other.state
    }
}

// ── UserData message ────────────────────────────────────────────────────────
/// An M2PA User Data message (sent on SCTP stream 1), carrying an MTP3 MSU.
#[pyclass(name = "UserData", module = "m2pa._m2pa", skip_from_py_object)]
#[derive(Clone)]
pub struct PyUserData {
    #[pyo3(get)]
    pub bsn: u32,
    #[pyo3(get)]
    pub fsn: u32,
    #[pyo3(get)]
    pub priority: u8,
    msu: Vec<u8>,
}

#[pymethods]
impl PyUserData {
    #[new]
    #[pyo3(signature = (msu, *, priority = 0, bsn = 0, fsn = 0))]
    fn new(msu: Vec<u8>, priority: u8, bsn: u32, fsn: u32) -> Self {
        // Core masks priority to its 2 valid bits; mirror that here.
        Self {
            bsn,
            fsn,
            priority: priority & 0x03,
            msu,
        }
    }

    /// The MTP3 MSU payload as `bytes`.
    #[getter]
    fn msu<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.msu)
    }

    /// Encode the complete M2PA message (common header + M2PA header + body).
    fn encode<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let msg = M2paMessage::UserData {
            bsn: self.bsn,
            fsn: self.fsn,
            message: UserDataMessage::new(self.priority, self.msu.clone()),
        };
        let bytes = msg.encode().map_err(m2pa_err)?;
        Ok(PyBytes::new(py, &bytes))
    }

    fn __repr__(&self) -> String {
        format!(
            "UserData(priority={}, bsn={}, fsn={}, msu_len={})",
            self.priority,
            self.bsn,
            self.fsn,
            self.msu.len()
        )
    }
}

// ── State machine ───────────────────────────────────────────────────────────
/// The RFC 4165 §4 link state machine: feed it received `LinkState`s and it
/// tracks the local link state (`OutOfService` → … → `InService`).
#[pyclass(name = "StateMachine", module = "m2pa._m2pa")]
pub struct PyStateMachine {
    inner: M2paStateMachine,
}

#[pymethods]
impl PyStateMachine {
    #[new]
    fn new() -> Self {
        Self {
            inner: M2paStateMachine::new(),
        }
    }

    /// The current link state.
    #[getter]
    fn state(&self) -> PyM2paState {
        PyM2paState::from_core(self.inner.state())
    }

    /// Begin alignment from `OutOfService`. Returns `True` if the transition was
    /// valid (i.e. the link was out of service).
    fn start(&mut self) -> bool {
        self.inner.start()
    }

    /// Feed a received Link Status and return the resulting local state.
    fn on_link_status(&mut self, link_state: PyLinkState) -> PyM2paState {
        PyM2paState::from_core(self.inner.on_link_status(link_state.to_core()))
    }

    /// Force the link out of service.
    fn stop(&mut self) {
        self.inner.stop();
    }

    fn __repr__(&self) -> String {
        format!("StateMachine(state={})", self.inner.state())
    }
}

// ── decode() ────────────────────────────────────────────────────────────────
/// Decode a complete M2PA message, returning a [`LinkStatus`] or [`UserData`].
#[pyfunction]
fn decode(py: Python<'_>, data: &[u8]) -> PyResult<Py<PyAny>> {
    let msg = M2paMessage::decode(data).map_err(m2pa_err)?;
    match msg {
        M2paMessage::LinkStatus { bsn, fsn, message } => {
            let obj = Bound::new(
                py,
                PyLinkStatus {
                    bsn,
                    fsn,
                    state: PyLinkState::from_core(message.state),
                },
            )?;
            Ok(obj.into_any().unbind())
        }
        M2paMessage::UserData { bsn, fsn, message } => {
            let obj = Bound::new(
                py,
                PyUserData {
                    bsn,
                    fsn,
                    priority: message.priority,
                    msu: message.msu,
                },
            )?;
            Ok(obj.into_any().unbind())
        }
    }
}

// ── Module wiring ───────────────────────────────────────────────────────────
fn add_contents(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("M2paError", m.py().get_type::<M2paError>())?;
    m.add_class::<PyLinkState>()?;
    m.add_class::<PyM2paState>()?;
    m.add_class::<PyLinkStatus>()?;
    m.add_class::<PyUserData>()?;
    m.add_class::<PyStateMachine>()?;
    m.add_function(wrap_pyfunction!(decode, m)?)?;
    // Protocol constants (RFC 4165 §2).
    m.add("VERSION", VERSION)?;
    m.add("MESSAGE_CLASS", MESSAGE_CLASS_M2PA)?;
    m.add("MESSAGE_TYPE_USER_DATA", MESSAGE_TYPE_USER_DATA)?;
    m.add("MESSAGE_TYPE_LINK_STATUS", MESSAGE_TYPE_LINK_STATUS)?;
    m.add("SCTP_PPID", SCTP_PPID)?;
    Ok(())
}

/// Standalone wheel entry point (maturin `module-name = "m2pa._m2pa"`).
#[pymodule]
fn _m2pa(m: &Bound<'_, PyModule>) -> PyResult<()> {
    add_contents(m)
}

/// Embedding entry point: build an `m2pa` submodule and attach it to `parent`,
/// so a host extension can expose m2pa without a second shared object.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "m2pa")?;
    add_contents(&m)?;
    parent.setattr("m2pa", &m)?;
    Ok(())
}
