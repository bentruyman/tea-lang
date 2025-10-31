# Native Intrinsics

Native intrinsics are special functions implemented in Rust that provide low-level functionality not expressible in Tea itself. They are prefixed with `__intrinsic_` to distinguish them from regular functions.

## Design Principles

1. **Minimal Surface**: Only functions that absolutely require native implementation
2. **Stable ABI**: Intrinsic signatures should remain stable across versions
3. **Type Safety**: All intrinsics validate argument types and counts
4. **Error Handling**: Native errors propagate as Tea runtime errors

## Core Intrinsics

### Type Predicates

- `__intrinsic_is_nil(value) -> bool`
- `__intrinsic_is_bool(value) -> bool`
- `__intrinsic_is_int(value) -> bool`
- `__intrinsic_is_float(value) -> bool`
- `__intrinsic_is_string(value) -> bool`
- `__intrinsic_is_list(value) -> bool`
- `__intrinsic_is_struct(value) -> bool`
- `__intrinsic_is_error(value) -> bool`

### Conversion

- `__intrinsic_to_string(value) -> string`

### Assertions

- `__intrinsic_fail(message: string) -> never`
- `__intrinsic_assert_snapshot(name: string, value: any, path: string) -> void`

### Environment (std.env)

- `__intrinsic_env_get(key: string) -> string?`
- `__intrinsic_env_set(key: string, value: string) -> void`
- `__intrinsic_env_unset(key: string) -> void`
- `__intrinsic_env_has(key: string) -> bool`
- `__intrinsic_env_vars() -> dict`
- `__intrinsic_env_cwd() -> string`
- `__intrinsic_env_set_cwd(path: string) -> void`
- `__intrinsic_env_temp_dir() -> string`
- `__intrinsic_env_home_dir() -> string`
- `__intrinsic_env_config_dir() -> string`

### Filesystem (std.fs)

- `__intrinsic_fs_read_text(path: string) -> string`
- `__intrinsic_fs_write_text(path: string, content: string) -> void`
- `__intrinsic_fs_write_text_atomic(path: string, content: string) -> void`
- `__intrinsic_fs_read_bytes(path: string) -> list[int]`
- `__intrinsic_fs_write_bytes(path: string, bytes: list[int]) -> void`
- `__intrinsic_fs_write_bytes_atomic(path: string, bytes: list[int]) -> void`
- `__intrinsic_fs_create_dir(path: string, recursive: bool) -> void`
- `__intrinsic_fs_remove(path: string) -> void`
- `__intrinsic_fs_exists(path: string) -> bool`
- `__intrinsic_fs_is_dir(path: string) -> bool`
- `__intrinsic_fs_is_symlink(path: string) -> bool`
- `__intrinsic_fs_size(path: string) -> int`
- `__intrinsic_fs_modified(path: string) -> int`
- `__intrinsic_fs_permissions(path: string) -> int`
- `__intrinsic_fs_is_readonly(path: string) -> bool`
- `__intrinsic_fs_list_dir(path: string) -> list[string]`
- `__intrinsic_fs_walk(path: string) -> list[string]`
- `__intrinsic_fs_glob(pattern: string) -> list[string]`
- `__intrinsic_fs_metadata(path: string) -> dict`
- `__intrinsic_fs_open_read(path: string) -> int`
- `__intrinsic_fs_read_chunk(handle: int, size: int) -> list[int]`
- `__intrinsic_fs_close(handle: int) -> void`

### Path (std.path)

- `__intrinsic_path_join(...parts: string) -> string`
- `__intrinsic_path_components(path: string) -> list[string]`
- `__intrinsic_path_dirname(path: string) -> string`
- `__intrinsic_path_basename(path: string) -> string`
- `__intrinsic_path_extension(path: string) -> string`
- `__intrinsic_path_set_extension(path: string, ext: string) -> string`
- `__intrinsic_path_strip_extension(path: string) -> string`
- `__intrinsic_path_normalize(path: string) -> string`
- `__intrinsic_path_absolute(path: string) -> string`
- `__intrinsic_path_relative(from: string, to: string) -> string`
- `__intrinsic_path_is_absolute(path: string) -> bool`
- `__intrinsic_path_separator() -> string`

### I/O (std.io)

- `__intrinsic_io_read_line() -> string`
- `__intrinsic_io_read_all() -> string`
- `__intrinsic_io_read_bytes() -> list[int]`
- `__intrinsic_io_write(text: string) -> void`
- `__intrinsic_io_write_err(text: string) -> void`
- `__intrinsic_io_flush() -> void`

### Process (std.process)

- `__intrinsic_process_run(cmd: string, args: list[string]) -> dict`
- `__intrinsic_process_spawn(cmd: string, args: list[string]) -> int`
- `__intrinsic_process_wait(handle: int) -> int`
- `__intrinsic_process_kill(handle: int) -> void`
- `__intrinsic_process_read_stdout(handle: int) -> string`
- `__intrinsic_process_read_stderr(handle: int) -> string`
- `__intrinsic_process_write_stdin(handle: int, text: string) -> void`
- `__intrinsic_process_close_stdin(handle: int) -> void`
- `__intrinsic_process_close(handle: int) -> void`

### Codecs (std.json, std.yaml)

- `__intrinsic_json_encode(value: any) -> string`
- `__intrinsic_json_decode(text: string) -> any`
- `__intrinsic_yaml_encode(value: any) -> string`
- `__intrinsic_yaml_decode(text: string) -> any`

### CLI (std.cli)

- `__intrinsic_cli_args() -> list[string]`
- `__intrinsic_cli_parse(spec: dict) -> dict`
- `__intrinsic_cli_capture(fn: function) -> string`

## Implementation Notes

- Intrinsics are resolved at compile-time by the resolver
- They bypass normal function call overhead in the VM
- Type checking occurs at both compile-time (when possible) and runtime
- All intrinsics that perform I/O or system calls can fail with runtime errors
