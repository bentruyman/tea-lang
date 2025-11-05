# Standard Library Removals

I want to trim down the standard library in tea-lang. Right now it has a lot of unnecessary modules and methods, and some things that need to be thought through in more detail. Below lists modules/methods that need to be removed from the Tea standard library.

## cli

- Remove the entire module. It's a higher-level module that's better served by a `support.cli` Tea package.

## env

- get_or
- require
- set_cwd
- temp_dir
- home_dir
- config_dir
- require_all
- has_any
- has_all
- get_first
- is_true
- is_false

## fs

- write_text_atomic
- ensure_parent
- is_dir
- is_symlink
- size
- modified
- permissions
- is_readonly
- glob
- metadata
- is_file
- has_extension
- filter_by_extension
- filter_files
- filter_dirs
- read_text_or
- write_text_safe
- is_empty_dir
- remove_if_exists

## io

- Remove the entire module. I want a better interface for streaming I/O. Maybe some kind of "Writer" interface. But for now, let's just remove it.

## math

- is_even
- is_odd
- clamp

## path

- strip_extension
- set_extension
- is_absolute

## process

- Remove the entire module. It has overlap with `std.io` and we need a better streaming interface. Will handle that later.

## string

- substring
- char_at
- parse_int
- truncate
- indent
- to_lower_ascii
- to_upper_ascii

## util

- Remove the entire module. Everything in this module is either handled by the language, the global `length` function, with the exception of `to_string` which we'll make a global later.
