# todo

A simple command-line task manager written in Tea.

## Description

A persistent todo list application that stores tasks in a text file. Supports adding, listing, completing, and removing tasks. Each task has a unique ID that persists across sessions.

## Usage

```bash
tea examples/todo/main.tea <command> [arguments]
```

### Commands

| Command                  | Description                      |
| ------------------------ | -------------------------------- |
| `init`                   | Initialize the todo file         |
| `add <task>`             | Add a new task                   |
| `list`, `ls`             | List all tasks with their status |
| `done <id>`              | Mark a task as complete          |
| `remove <id>`, `rm <id>` | Delete a task                    |
| `help`                   | Show help message                |

### Environment Variables

- `TODO_FILE` — Path to the todo file (default: `~/.todo.txt`)

## Examples

```bash
# Initialize the todo file (required first time)
tea examples/todo/main.tea init

# Add tasks
tea examples/todo/main.tea add Write documentation
tea examples/todo/main.tea add "Review pull request"
tea examples/todo/main.tea add Fix the login bug

# List all tasks
tea examples/todo/main.tea list
# Output:
# Tasks:
#
#   1. [ ] Write documentation
#   2. [ ] Review pull request
#   3. [ ] Fix the login bug
#
# 0/3 completed

# Mark a task as done
tea examples/todo/main.tea done 2
# Output: Completed: Review pull request

# List again
tea examples/todo/main.tea list
# Output:
# Tasks:
#
#   1. [ ] Write documentation
#   2. [x] Review pull request
#   3. [ ] Fix the login bug
#
# 1/3 completed

# Remove a task
tea examples/todo/main.tea remove 1
# Output: Removed: Write documentation

# Use a custom file location
TODO_FILE=/tmp/work-tasks.txt tea examples/todo/main.tea init
TODO_FILE=/tmp/work-tasks.txt tea examples/todo/main.tea add "Work task"
```

## Data Format

Tasks are stored in a simple text format at `~/.todo.txt` (or `$TODO_FILE`):

```
ID|DONE|TEXT
```

Example:

```
1|0|Write documentation
2|1|Review pull request
3|0|Fix the login bug
```

Where `DONE` is `0` for incomplete and `1` for complete.

## Tea Features Demonstrated

- **`std.args`** — Subcommand parsing with positional arguments
- **`std.fs`** — Reading and writing text files
- **`std.env`** — Environment variable access
- **`std.path`** — Path manipulation (joining paths)
- **String interpolation** — Formatted output with `` `${var}` ``
- **Control flow** — Subcommand dispatch with if/else chains
- **Error handling** — Input validation and meaningful error messages
- **String parsing** — Manual parsing of pipe-delimited format

## Build

Compile to a native binary for faster execution:

```bash
tea build examples/todo/main.tea -o todo
./todo init
./todo add "My first task"
./todo list
```

## Limitations

- No due dates or priorities
- No task editing (remove and re-add instead)
- No search or filtering
- Single-user only (no locking for concurrent access)
- Todo file must be initialized before first use
