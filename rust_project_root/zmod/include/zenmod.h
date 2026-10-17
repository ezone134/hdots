/* zenmod.h — the zen-shell data-provider plugin ABI, version 1.
 *
 * A plugin is a shared object dropped in $sources/lib/. The shell dlopens it
 * once, asks for its version, then pulls (key, value) pairs every refresh
 * tick. Values land in the same namespace as the shell's built-in scene
 * values, so a .ron card binds them with the usual `{name}` interpolation:
 *
 *     Header(title: "Clock", meta: Some("{mod_clock.time}"))
 *
 * Rules for plugin authors
 * ------------------------
 *  - The four functions must have C linkage and exact signatures below.
 *  - Return NUL-terminated UTF-8 from the two getters. The host copies the
 *    bytes immediately, so the pointer only has to stay valid until your
 *    function returns.
 *  - Keys are dotted lowercase names, `mod_<module>.<name>`, and must not
 *    contain whitespace, `{` or `}` — braces would be read as scene-value
 *    interpolation by the card that binds them.
 *  - NEVER let a panic escape these functions. In Rust, an unwind out of an
 *    `extern "C"` fn aborts the process (since 1.71), and the host cannot
 *    catch it for you: catch panics inside your own getters and return a
 *    fallback value instead. In C/C++ the same applies to exceptions.
 *  - The host never calls dlclose(), so your static constructors and
 *    destructors stay valid for the life of the shell.
 *
 * Writing a plugin in Rust: `crate-type = ["cdylib"]`, `panic = "unwind"`,
 * and wrap each getter in `std::panic::catch_unwind` — see zmods/clock.
 */
#ifndef ZENMOD_H
#define ZENMOD_H

#ifdef __cplusplus
extern "C" {
#endif

/* Bumped whenever the function signatures change. The host refuses to load a
 * module whose version does not match, so an old .so can never be called with
 * new expectations. */
#define ZEN_ABI_VERSION 1u

/* Must return ZEN_ABI_VERSION. */
unsigned int zen_abi_version(void);

/* How many (key, value) pairs this module publishes. Keep it small: the host
 * caps this at ZEN_MAX_PROVIDERS and reports an error above that. */
unsigned int zen_provider_count(void);

/* NUL-terminated key for slot i, e.g. "mod_clock.time". NULL is treated as
 * an error for that slot. */
const char *zen_provider_key(unsigned int i);

/* NUL-terminated value for slot i, re-computed on every call — the host pulls
 * all values once per tick, so a clock just formats the current time here. */
const char *zen_provider_value(unsigned int i);

/* Host-side sanity limits, exposed so plugins can be tested against the same
 * numbers the shell enforces. */
#define ZEN_MAX_PROVIDERS 4096u
#define ZEN_MAX_KEY 128u
#define ZEN_MAX_VALUE 65536u

#ifdef __cplusplus
}
#endif

#endif /* ZENMOD_H */
