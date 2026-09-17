//! A minimal LADSPA wrapper around our DSP.
//!
//! LADSPA is the little contract that lets audio hosts (PipeWire) load a
//! `.so`, ask for a "plugin", connect wires to its ports, and then just call
//! `run(sample_count)` over and over. We implement that contract here and
//! forward the actual work to `Equalizer`.
//!
//! Ports on our plugin, in order:
//!   0  Input           (audio, feed this)
//!   1  Output          (audio, read this)
//!   2..11  the ten band gains in dB
//!   12 Master          (master gain in dB)
//!
//! LADSPA does the channel handling: PipeWire creates one *instance* of us
//! per audio channel, all sharing the same control values.

use core::ptr::null_mut;
use std::ffi::c_char;
use std::ffi::c_void;
use std::sync::OnceLock;

use crate::eq::Equalizer;

// ---------------------------------------------------------------------------
// Port constants (matching the LADSPA spec)
// ---------------------------------------------------------------------------

const PORT_INPUT: u64 = 0x1; // LADSPA_PORT_INPUT
const PORT_OUTPUT: u64 = 0x2; // LADSPA_PORT_OUTPUT
const PORT_CONTROL: u64 = 0x4; // LADSPA_PORT_CONTROL
const PORT_AUDIO: u64 = 0x8; // LADSPA_PORT_AUDIO

const HINT_BOUNDED_BELOW: u64 = 0x1;
const HINT_BOUNDED_ABOVE: u64 = 0x2;
const HINT_DEFAULT_0: u64 = 0x40;

pub const P_INPUT: usize = 0;
pub const P_OUTPUT: usize = 1;
pub const P_GAIN_BASE: usize = 2;
pub const P_MASTER: usize = 12;
pub const PORT_COUNT: usize = 13;
pub const NUM_BANDS: usize = 10;

// ---------------------------------------------------------------------------
// The LADSPA_Descriptor struct, laid out exactly like the C header so the
// host can read it. We only need the fields we actually use.
// ---------------------------------------------------------------------------

type Handle = *mut c_void;

#[repr(C)]
pub struct LADSPA_Descriptor {
    unique_id: u64,
    label: *const c_char,
    name: *const c_char,
    maker: *const c_char,
    copyright: *const c_char,
    port_count: u64,
    port_descriptors: *const u64,
    port_names: *const *const c_char,
    port_range_hints: *const PortRangeHint,
    implementation_data: *const c_void,
    instantiate: extern "C" fn(*const LADSPA_Descriptor, u64) -> Handle,
    connect_port: extern "C" fn(Handle, u64, *mut f32),
    activate: extern "C" fn(Handle),
    run: extern "C" fn(Handle, u64),
    // run_adding / set_run_adding_gain are optional → NULL.
    run_adding: Option<extern "C" fn(Handle, u64, f32)>,
    set_run_adding_gain: Option<extern "C" fn(Handle, f32)>,
    deactivate: extern "C" fn(Handle),
    cleanup: extern "C" fn(Handle),
}

#[repr(C)]
pub struct PortRangeHint {
    hint_descriptor: u64,
    lower_bound: f32,
    upper_bound: f32,
}

// ---------------------------------------------------------------------------
// The per-channel instance. Host allocates one of these per audio channel.
// ---------------------------------------------------------------------------

pub struct EqInstance {
    pub eq: Equalizer,
    /// Where the host wants us to read/write samples. Null until connected.
    ports: [*mut f32; PORT_COUNT],
}

impl EqInstance {
    fn new(sample_rate: u32) -> Self {
        Self { eq: Equalizer::new(sample_rate), ports: [null_mut(); PORT_COUNT] }
    }
}

// ---------------------------------------------------------------------------
// The six LADSPA lifecycle functions. These are the actual ABI.
// ---------------------------------------------------------------------------

extern "C" fn instantiate(_desc: *const LADSPA_Descriptor, sample_rate: u64) -> Handle {
    let inst = Box::new(EqInstance::new(sample_rate as u32));
    Box::into_raw(inst) as Handle
}

extern "C" fn connect_port(handle: Handle, port: u64, location: *mut f32) {
    let inst = unsafe { &mut *(handle as *mut EqInstance) };
    if let Some(slot) = inst.ports.get_mut(port as usize) {
        *slot = location;
    }
}

extern "C" fn activate(handle: Handle) {
    let inst = unsafe { &mut *(handle as *mut EqInstance) };
    inst.eq.reset();
}

extern "C" fn deactivate(_handle: Handle) {}

extern "C" fn run(handle: Handle, sample_count: u64) {
    let inst = unsafe { &mut *(handle as *mut EqInstance) };
    let input = inst.ports[P_INPUT];
    let output = inst.ports[P_OUTPUT];
    // Safety: these are only dereferenced if the host connected both ports,
    // and the host is required to do that before calling run().
    if input.is_null() || output.is_null() {
        return;
    }

    // Pull the control values the host set for us this run. Null slots just
    // read as 0 dB (flat) — LADSPA lets hosts skip connecting unused ports.
    let mut gains = [0.0f32; NUM_BANDS];
    for (slot, g) in inst.ports.iter().skip(P_GAIN_BASE).take(NUM_BANDS).zip(&mut gains) {
        if !(*slot).is_null() {
            *g = unsafe { **slot };
        }
    }
    let master = if inst.ports[P_MASTER].is_null() {
        0.0
    } else {
        unsafe { *inst.ports[P_MASTER] }
    };
    inst.eq.set_gains(&gains, master);

    // The juicy bit: push every sample through our ten-band DSP.
    for i in 0..sample_count {
        let x = unsafe { *input.add(i as usize) };
        unsafe { *output.add(i as usize) = inst.eq.process(x) };
    }
}

extern "C" fn cleanup(handle: Handle) {
    // Recover the Box we created in instantiate() and let it drop for real.
    unsafe { drop(Box::from_raw(handle as *mut EqInstance)) };
}

// ---------------------------------------------------------------------------
// The descriptor: the recipe the host reads to discover our ports and
// lifecycle functions. Built once and leaked so its address never changes.
// ---------------------------------------------------------------------------

/// Raw pointers aren't `Send`/`Sync` by default, but ours is an immutable,
/// process-lifetime object — safe to share once built.
#[derive(Clone, Copy)]
struct DescPtr(*const LADSPA_Descriptor);
unsafe impl Send for DescPtr {}
unsafe impl Sync for DescPtr {}

#[no_mangle]
pub extern "C" fn ladspa_descriptor(index: u32) -> *const LADSPA_Descriptor {
    if index != 0 {
        return core::ptr::null();
    }
    static DESCRIPTOR: OnceLock<DescPtr> = OnceLock::new();
    DESCRIPTOR.get_or_init(|| DescPtr(build_descriptor())).0
}

const PLUGIN_LABEL: &str = "zeneq";

fn build_descriptor() -> *const LADSPA_Descriptor {
    let port_descriptors: Vec<u64> = std::iter::once(PORT_INPUT | PORT_AUDIO)
        .chain(std::iter::once(PORT_OUTPUT | PORT_AUDIO))
        .chain(std::iter::repeat_n(PORT_INPUT | PORT_CONTROL, NUM_BANDS))
        .chain(std::iter::once(PORT_INPUT | PORT_CONTROL))
        .collect();

    let band_names = [
        "31 Hz", "62 Hz", "125 Hz", "250 Hz", "500 Hz", "1 kHz", "2 kHz", "4 kHz", "8 kHz",
        "16 kHz",
    ];
    let port_names: Vec<*const c_char> = vec!["Input", "Output"]
        .into_iter()
        .chain(band_names)
        .chain(std::iter::once("Master"))
        .map(cstr)
        .collect();

    let port_range_hints: Vec<PortRangeHint> = port_names
        .iter()
        .map(|_| PortRangeHint {
            hint_descriptor: HINT_BOUNDED_BELOW | HINT_BOUNDED_ABOVE | HINT_DEFAULT_0,
            lower_bound: -12.0,
            upper_bound: 12.0,
        })
        .collect();

    let (names, makers, copyrights) = (cstr(PLUGIN_LABEL), cstr("zeneq"), cstr("zeneq"));

    let desc = Box::new(LADSPA_Descriptor {
        unique_id: 2718,
        label: names,
        name: cstr("zen-shell 10-band Equalizer"),
        maker: makers,
        copyright: copyrights,
        port_count: PORT_COUNT as u64,
        port_descriptors: box_slice(port_descriptors),
        port_names: box_slice(port_names),
        port_range_hints: box_slice(port_range_hints),
        implementation_data: core::ptr::null(),
        instantiate: instantiate as extern "C" fn(_, _) -> _,
        connect_port: connect_port as extern "C" fn(_, _, _),
        activate: activate as extern "C" fn(_),
        run: run as extern "C" fn(_, _),
        run_adding: None,
        set_run_adding_gain: None,
        deactivate: deactivate as extern "C" fn(_),
        cleanup: cleanup as extern "C" fn(_),
    });
    Box::into_raw(desc) as *const LADSPA_Descriptor
}

/// Build a NUL-terminated C string pointer. Leaks: it lives forever, which is
/// fine for data the host keeps for the whole session.
fn cstr(s: &str) -> *const c_char {
    let mut bytes = s.as_bytes().to_vec();
    bytes.push(0);
    // `as *const c_char` converts the byte slice pointer to a C string
    // pointer. NUL-terminated + lives forever = a valid C string.
    Box::leak(bytes.into_boxed_slice()).as_ptr() as *const c_char
}

fn box_slice<T>(v: Vec<T>) -> *const T {
    Box::leak(v.into_boxed_slice()).as_ptr()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    /// Drive the plugin exactly the way a host would: get the descriptor,
    /// instantiate with a sample rate (also discover the sample rate for a
    /// test signal), connect ports, activate, run, cleanup.
    #[test]
    fn plugin_runs_like_a_host_would() {
        let desc = ladspa_descriptor(0);
        assert!(!desc.is_null());
        let desc = unsafe { &*desc };
        assert_eq!(desc.port_count, PORT_COUNT as u64);

        const SAMPLE_RATE: u32 = 48000;
        const N: usize = 8192;

        let handle = (desc.instantiate)(desc, SAMPLE_RATE as u64);
        let inst = unsafe { &mut *(handle as *mut EqInstance) };

        let mut input = vec![0.0f32; N];
        let mut output = vec![0.0f32; N];
        // A tone right at the first band's centre (31 Hz) so the +9 dB boost
        // lands squarely on it.
        for i in 0..N {
            input[i] = (2.0 * PI * (i as f64) * 31.0 / SAMPLE_RATE as f64).sin() as f32;
        }

        (desc.connect_port)(handle, P_INPUT as u64, input.as_mut_ptr());
        (desc.connect_port)(handle, P_OUTPUT as u64, output.as_mut_ptr());

        // Lift the 31 Hz band to +9 dB — port 2, right after the two audio ports.
        let mut band_gain = 9.0f32;
        let mut master = 0.0f32;
        (desc.connect_port)(handle, P_GAIN_BASE as u64, &mut band_gain);
        (desc.connect_port)(handle, P_MASTER as u64, &mut master);

        (desc.activate)(handle);
        (desc.run)(handle, N as u64);

        // Steady-state loudness, skipping the early transient.
        let peak_in = input[N / 2..].iter().fold(0.0f32, |m, x| m.max(x.abs()));
        let peak_out = output[N / 2..].iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak_out > peak_in * 1.8, "expected boost, got {peak_out} vs {peak_in}");

        (desc.cleanup)(handle);
    }
}