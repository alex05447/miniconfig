# miniconfig

A minimalistic config file library written in Rust.

## **Overview**

Config (file)s here are meant to be collections of key - value pairs. Think JSON.

Primitive value types are
- booleans,
- integers (signed, 64-bit),
- floats (double precision / 64 bit),
- strings (UTF-8).

Primitive values may be contained in
- tables / hash maps / objects etc., with (non-empty, UTF-8) string keys,
- arrays / lists / etc. (with elements of homogenous type, with `0`-based contiguous integer indices).

Tables and arrays may contain nested tables and arrays
(except `.ini` configs (requires `"ini"` feature) which only support arrays of primitive types and do not support nested tables).

Each config has a (possibly empty) `root` table.

### **(Table, incl. root table) keys**

Any valid UTF-8 string, with special characters escaped.

These are the special characters which must always be escaped:

- `\`
- `\0`
- `\a`
- `\b`
- `\t`
- `\n`
- `\v`
- `\f`
- `\r`

#### **Lua**

In Lua configs (requires `"lua"` feature), keys work according to Lua rules: keys which are not valid Lua identifiers (i.e. do not contain only from ASCII alphanumerical characters and underscores and start with an ASCII alphabetical character) must be enclosed in brackets and quotes (`"` \ `'`) (e.g. `["áéíóú"]`). Within quoted strings, enclosed in (matching) single (`'`) or double (`"`) quotes, non-matching double (`"`) or single (`'`) quotes and spaces (`' '`) don't have to be escaped. Unicode 2-digit hexadecimal escape sequences work according to Lua rules.

#### **INI**

In `.ini` configs (requires `"ini"` feature), additionally, special `.ini` characters and spaces (`' '`) must be escaped in section names and keys.

These are the special `.ini` characters:

- `[` (section start delimiter, optional array start delimiter)
- `]` (section end delimiter, optional array end delimiter)
- `;` (default comment delimiter)
- `#` (optional comment delimiter)
- `=` (default key-value separator)
- `:` (optional key-value separator)

Section names, keys and string values may be enclosed in (matching) single (`'`) or double (`"`) quotes. In this case spaces (`' '`), non-matching double (`"`) or single (`'`) quotes and special `.ini` characters do not have to be (but may be) escaped.

### **Values**

For string values the rules are the same as for keys.

Strings `"true"` and `"false"` (case-sensitive) are the only valid boolean value representations (i.e. not `"on"` / `"off"`, `"yes"` / `"no"`, `"TRUE"` / `"FALSE"`, `"0"` / `"1"`).

In Lua configs (requires `"lua"` feature), integer and float values work according to Lua rules. String values are always quoted in (matching) single (`'`) or (`"`) double quotes.

In `.ini` configs (requires `"ini"` feature), integer and float values work according to Rust integer / float parsing rules. Quoted values are always parsed as strings; otherwise values are first parsed as booleans, than as integers and lastly as floats.

## **Lua configs** (requires `"lua"` feature).

Main format for human-readable config files with nested array/table support.

Piggybacks on the Lua interpreter both as a parser and as runtime data representation.

May be used directly as a Lua representation within an [`rlua Context`](http://docs.rs/rlua/*/rlua/struct.Context.html), or be serialized for dynamic (requires `"dyn"` feature) or read-only (requires `"bin"` feature) use to decouple itself from the Lua state.

**Data**: text file representing a(n incomplete) Lua script, declaring a root config table with string keys, including nested config arrays/tables represented by Lua tables. Only a subset of Lua types / features are supported.

**Runtime**: internally represented by a root Lua table reference. Provides a mutable config interface. Can add/modify/remove values.

**Serialization**: to string Lua script, to binary config (requires `"bin"` feature), to string `.ini` config (requires `"ini"` feature, does not support non-primitive arrays and nested tables), to "dynamic" config (requires `"dyn"` feature).

**Example**:

``` lua
{
    array_value = { 54, 12, 78.9 }, -- array_value
    bool_value = true,
    float_value = 3.14,
    int_value = 7,
    string_value = "foo",
    table_value = {
        bar = 2020,
        baz = "hello",
        foo = false,
    }, -- table_value
}
```

**Use cases**: use Lua config source text files for human-readable / writable / mergeable / diff-able data of arbitrary complexity that frequently changes during development, but does not need to / must not be user-visible.

## **"Dynamic" configs** (requires `"dyn"` feature).

Based on Rust hash maps and arrays.

Main format for runtime representation of dynamic configs, or an intermediate representation for Lua configs (after deserialization) / binary configs (before serialization).

**Data**: if `"ini"` feature is enabled - a text file representing a valid `.ini` config, declaring a root config table with string keys and a number of sections a.k.a tables. Does not support non-primitive arrays and nested tables.

**Runtime**: internally represented by a root Rust hash map with string keys; arrays are Rust vectors. Provides a mutable config interface. Can add/modify/remove values.

**Serialization**: to string Lua script (requires `"lua"` feature), to binary config (requires `"bin"` feature), to string `.ini` config (requires `"ini"` feature, does not support non-primitive arrays and nested tables).

**Example**:

```ini
; Semicolon starts a line comment by default.
# Optionally you may use the number sign for a line comment.

; This and following key/value pairs go to the root of the config.
; Unquoted `value` is parsed as a string if support for unquoted strings is enabled
; (it is by default).
key = value ; Inline comments are optionally supported.

; Spaces and other special / `.ini` characters may be escaped with `\`.
; This key is `key 2`, value is a boolean `true`.
; The only valid values for booleans are the strings `true` and `false`
; (but not `yes` \ `no`, `on` \ `off`, `0` \ `1`).
key\ 2 = true

; Quoted keys do not have to escape space and `.ini` characters
; (but do have to escape special characters).
; Double quotes (`"`) are used by default, single quotes (`'`) are optional.
; This key is `key 3`, value is a signed 64-buit integer `7`.
"key 3" = 7

; Sections declare tables with string keys
; and boolean/integer/floating/array values.
; All following key/value pairs go to this section.
; Sections may be empty.
; Section names are enclosed in brackets (`[` \ `]`).
; Leading and trailing whitespace is ignored.
; This section name is `some_section`, not ` some_section `
; (note the skipped spaces).
[ some_section ]

; 4 hexadecimal digit Unicode escape sequences are supported.
; This key is `foo`.
; Quoted values (in single quotes this time) are always parsed as strings.
; Non-matching quotes (double quotes here) don't have to be escaped.
; This value is a string `"42"` (not an integer).
\x0066\x006f\x006f = '"42"'

; Colon (`:`) is supported as an optional key-value separator.
; This key is `bar`, value is a 64-bit floating point value `3.14`.
bar : 3.14

; Section names may be enclosed in quotes; same rules as keys.
["other section"]

; Arrays are optionally supported.
; Array values are enclosed in brackets (`[` \ `]`)
; and are delimited by commas `,`.
; Trailing commas are allowed.
; Arrays may only contain boolean/integer/float/string **values**
; and only values of the same type (except ints and floats, which may be mixed).
; This array contains two ints and a float, which may be interpreted as both types.
; If you query them as ints, you'll get `[3, 4, 7]`.
; If you query them as floats, you'll get `[3.0, 4.0, 7.62]`.
'array value' = [3, 4, 7.62,]

; Duplicate sections are merged by default,
; but this behaviour may be configured.
["some_section"]

; Line continuations (backslash `\` followed by a new line)
; are optionally supported in section names, keys and values (including numeric values for what it's worth).
; This value is a string `a multiline string`.
; Section `some_section` now contains 3 keys - `foo`, `bar` and `baz`.
baz = a\
multiline\
string

; Duplicate keys within a section cause an error by default,
; but this behaviour may be configured
; to override the value with the new one,
; or ignore the new value.
baz = "an overridden value"

```

**Use cases**: if `"ini"` feature is enabled - use `.ini` config source text files for human-readable / writable data of limited complexity (only one level of nested tables) which must be user-visible/editable.

## **Binary configs** (requires `"bin"` feature).

Main format for on-disk / runtime representation of read-only configs with nested array/table support.

**Data**: raw byte blob. Generated by serializing a Lua config (requires `"lua"` feature), dynamic config (requires `"dyn"` feature), or by using the provided writer API directly.

In current implementation the data representation is slightly suboptimal in terms of size (e.g. arrays of primitive types are not stored optimally as there's some overhead per-element), but the benefits are implementation simplicity and the ability to distinguish between integers and floats even at array element granularity.

Strings (both keys and values) are deduplicated and stored separately in a contiguous blob. Stored strings are null-terminated.

**Runtime**: wrapper over the raw byte blob. Provides a read-only config interface. Cannot add/modify/remove values.

**Serialization**: to string Lua script (requires `"lua"` feature), to string `.ini` config (requires `"ini"` feature, does not support non-primitive arrays and nested tables).

**Use cases**: use for read-only data of arbitrary complexity which must not be user-visible, or for caching of data which does not need to change frequently at runtime for loading / access performance.

## **Examples**

See `example.rs`.

## **Building**

Requires some path dependencies in the parent directory - see `Dependencies` section.

## **Features**

The crate by itself with no features enabled exposes no functionality. Enable one or more of these:

- `"lua"` - adds support for Lua configs.
- `"dyn"` - adds support for dynamic configs.
- `"bin"` - adds support for binary configs, serialization of Lua/dynamic configs to binary configs.
- `"ini"` - adds support for parsing `.ini` config strings, deserialization to dynamic configs (requires `"dyn"` feature), serialization of Lua (requires `"lua"` feature) / dynamic (requires `"dyn"` feature) / binary (requires `"bin"` feature) configs to `.ini` config strings.

## **Dependencies**

- If `"lua"` feature is enabled (it is by default), [`rlua`](https://crates.io/crates/rlua) and [`rlua_ext`](https://github.com/alex05447/rlua_ext) as a path dependency (TODO - github dependency?).

- If `"ini"` feature is enabled, [`bitflags`](https://crates.io/crates/bitflags) for `.ini` parser options, and [`static_assertions`](https://crates.io/crates/static_assertions).

## **Problems / missing features**

Despite the fact that all configs implement a common interface, it is currently impossible to implement a Rust trait to encapsulate that
due to Rust not having GAT's (generic associated types) at the moment of writing.

As a result, some code is duplicated internally in the crate, and the crate users will not be able to write code generic over config implementations.
