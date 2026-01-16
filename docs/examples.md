# Examples and Common Patterns

This guide demonstrates practical Tea patterns and real-world examples. Use these as starting points for your own projects.

## Table of Contents

- [File Processing](#file-processing)
- [Command-Line Tools](#command-line-tools)
- [Data Processing](#data-processing)
- [Configuration Management](#configuration-management)
- [Testing](#testing)
- [Error Handling Patterns](#error-handling-patterns)

---

## File Processing

### Read and Process Text Files

```tea
use fs = "std.fs"
use string = "std.string"

def count_lines(file_path: String) -> Int
  var content = fs.read_file(file_path)
  var lines = string.split(content, "\n")
  @len(lines)
end

def process_log_file(path: String)
  var content = fs.read_file(path)
  var lines = string.split(content, "\n")
  var error_count = 0

  for line in lines
    if string.contains(line, "ERROR")
      error_count = error_count + 1
      @println(`ERROR: ${line}`)
    end
  end

  @println(`Total errors found: ${error_count}`)
end

process_log_file("app.log")
```

### Batch File Conversion

```tea
use fs = "std.fs"
use path = "std.path"
use string = "std.string"

def convert_all_files(input_dir: String, output_dir: String)
  var files = fs.read_dir(input_dir)

  # Ensure output directory exists
  if !fs.exists(output_dir)
    fs.create_dir(output_dir)
  end

  for file in files
    if string.ends_with(file, ".txt")
      var input_path = path.join([input_dir, file])
      var content = fs.read_file(input_path)

      # Process content
      var processed = string.to_upper(content)
      var trimmed = string.trim(processed)

      # Write output
      var output_file = string.replace(file, ".txt", ".processed.txt")
      var output_path = path.join([output_dir, output_file])
      fs.write_file(output_path, trimmed)

      @println(`Converted: ${file}`)
    end
  end
end

convert_all_files("input", "output")
```

### Directory Tree Walker

```tea
use fs = "std.fs"
use path = "std.path"
use string = "std.string"

def walk_directory(dir: String, extension: String) -> List[String]
  var results: List[String] = []
  var entries = fs.read_dir(dir)

  for entry in entries
    var full_path = path.join([dir, entry])

    if string.ends_with(entry, extension)
      # Add to results (in real code, append to list)
      @println(`Found: ${full_path}`)
    end
  end

  results
end

# Find all Tea source files
walk_directory("src", ".tea")
```

---

## Command-Line Tools

### Simple CLI Tool

```tea
use env = "std.env"
use fs = "std.fs"

def main()
  var args = env.args()

  if @len(args) < 2
    @println("Usage: tea script.tea <file>")
    return
  end

  var file_path = args[1]

  if !fs.exists(file_path)
    @println(`Error: File not found: ${file_path}`)
    return
  end

  var content = fs.read_file(file_path)
  @println(`File size: ${@len(content)} bytes`)
end

main()
```

### File Statistics Tool

```tea
use fs = "std.fs"
use string = "std.string"
use path = "std.path"

struct FileStats {
  path: String
  lines: Int
  words: Int
  chars: Int
}

def analyze_file(file_path: String) -> FileStats
  var content = fs.read_file(file_path)
  var lines = string.split(content, "\n")

  var word_count = 0
  for line in lines
    var words = string.split(string.trim(line), " ")
    word_count = word_count + @len(words)
  end

  FileStats(
    path: file_path,
    lines: @len(lines),
    words: word_count,
    chars: @len(content)
  )
end

def print_stats(stats: FileStats)
  @println(`File: ${stats.path}`)
  @println(`  Lines: ${stats.lines}`)
  @println(`  Words: ${stats.words}`)
  @println(`  Characters: ${stats.chars}`)
end

var stats = analyze_file("README.md")
print_stats(stats)
```

---

## Data Processing

### Filter and Transform Lists

```tea
def filter_even(numbers: List[Int]) -> List[Int]
  var result: List[Int] = []
  var index = 0

  for num in numbers
    if num % 2 == 0
      # In real code, append to result
      @println(num)
    end
  end

  result
end

def sum_list(numbers: List[Int]) -> Int
  var total = 0

  for num in numbers
    total = total + num
  end

  total
end

def average(numbers: List[Int]) -> Float
  if @len(numbers) == 0
    return 0.0
  end

  var sum = sum_list(numbers)
  @to_float(sum) / @to_float(@len(numbers))
end

var data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
var evens = filter_even(data)
var total = sum_list(data)
var avg = average(data)

@println(`Total: ${total}`)
@println(`Average: ${avg}`)
```

### Group and Aggregate Data

```tea
struct Sale {
  product: String
  amount: Float
  quantity: Int
}

def total_sales(sales: List[Sale]) -> Float
  var total = 0.0

  for sale in sales
    total = total + sale.amount
  end

  total
end

def total_quantity(sales: List[Sale]) -> Int
  var total = 0

  for sale in sales
    total = total + sale.quantity
  end

  total
end

var sales = [
  Sale(product: "Widget", amount: 25.00, quantity: 2),
  Sale(product: "Gadget", amount: 15.50, quantity: 1),
  Sale(product: "Widget", amount: 25.00, quantity: 3)
]

var revenue = total_sales(sales)
var items = total_quantity(sales)

@println(`Total Revenue: $${revenue}`)
@println(`Total Items Sold: ${items}`)
```

---

## Configuration Management

### JSON Configuration

```tea
use fs = "std.fs"
use json = "std.json"
use path = "std.path"
use env = "std.env"

struct Config {
  host: String
  port: String
  debug: Bool
}

def load_config(file_path: String) -> Config
  var json_str = fs.read_file(file_path)
  var data = json.decode(json_str)

  # Parse config data
  Config(
    host: data.host,
    port: data.port,
    debug: data.debug == "true"
  )
end

def save_config(file_path: String, config: Config)
  var data = {
    "host": config.host,
    "port": config.port,
    "debug": if(config.debug) "true" else "false"
  }

  var json_str = json.encode(data)
  fs.write_file(file_path, json_str)
end

def get_config_path() -> String
  var cwd = env.cwd()
  path.join([cwd, "config.json"])
end

# Load configuration
var config_path = get_config_path()

if fs.exists(config_path)
  var config = load_config(config_path)
  @println(`Loaded config: ${config.host}:${config.port}`)
else
  # Create default config
  var default_config = Config(
    host: "localhost",
    port: "8080",
    debug: false
  )
  save_config(config_path, default_config)
  @println("Created default configuration")
end
```

### Environment-Based Configuration

```tea
use env = "std.env"

struct AppConfig {
  environment: String
  log_level: String
  max_connections: Int
}

def load_from_env() -> AppConfig
  var env_name = env.get("APP_ENV") ?? "development"
  var log_level = env.get("LOG_LEVEL") ?? "info"
  var max_conn = env.get("MAX_CONNECTIONS") ?? "100"

  AppConfig(
    environment: env_name,
    log_level: log_level,
    max_connections: @parse_int(max_conn)
  )
end

var config = load_from_env()
@println(`Environment: ${config.environment}`)
@println(`Log Level: ${config.log_level}`)
```

---

## Testing

### Unit Tests with Assertions

```tea
use assert = "std.assert"

def add(a: Int, b: Int) -> Int
  a + b
end

def multiply(a: Int, b: Int) -> Int
  a * b
end

test "basic arithmetic"
  assert.eq(add(2, 3), 5)
  assert.eq(add(0, 0), 0)
  assert.eq(add(-1, 1), 0)
end

test "multiplication"
  assert.eq(multiply(2, 3), 6)
  assert.eq(multiply(0, 5), 0)
  assert.eq(multiply(-2, 3), -6)
end
```

### Testing String Functions

```tea
use assert = "std.assert"
use string = "std.string"

def normalize_whitespace(text: String) -> String
  var trimmed = string.trim(text)
  var lowercase = string.to_lower(trimmed)
  lowercase
end

test "normalize whitespace"
  var input = "  HELLO WORLD  "
  var result = normalize_whitespace(input)

  assert.eq(result, "hello world")
  assert.ok(string.starts_with(result, "hello"))
  assert.ok(string.ends_with(result, "world"))
end
```

### Snapshot Testing

```tea
use assert = "std.assert"

struct Report {
  title: String
  items: List[String]
}

def format_report(report: Report) -> String
  var output = `${report.title}\n`
  output = output + "==========\n"

  for item in report.items
    output = output + `- ${item}\n`
  end

  output
end

test "report formatting"
  var report = Report(
    title: "Daily Summary",
    items: ["Task 1", "Task 2", "Task 3"]
  )

  var formatted = format_report(report)
  assert.snapshot("daily_report", formatted)
end
```

---

## Error Handling Patterns

### Safe File Operations

```tea
use fs = "std.fs"

error FileOperationError {
  NotFound(path: String)
  AccessDenied(path: String)
  IOError(message: String)
}

def safe_read_file(path: String) -> String ! FileOperationError
  if !fs.exists(path)
    throw FileOperationError.NotFound(path)
  end

  # Actual read would check for access/IO errors
  fs.read_file(path)
end

def process_file(path: String)
  var content = safe_read_file(path) catch err
    case is FileOperationError.NotFound
      @println(`File not found: ${err.path}`)
      return
    case is FileOperationError.AccessDenied
      @println(`Access denied: ${err.path}`)
      return
    case _
      @println("Unknown error reading file")
      return
  end

  @println(`File size: ${@len(content)} bytes`)
end
```

### Validation with Custom Errors

```tea
error ValidationError {
  EmptyInput
  TooShort(min: Int, actual: Int)
  TooLong(max: Int, actual: Int)
  InvalidFormat(reason: String)
}

def validate_username(name: String) -> Bool ! ValidationError
  if @len(name) == 0
    throw ValidationError.EmptyInput
  end

  if @len(name) < 3
    throw ValidationError.TooShort(min: 3, actual: @len(name))
  end

  if @len(name) > 20
    throw ValidationError.TooLong(max: 20, actual: @len(name))
  end

  true
end

def create_account(username: String)
  var valid = validate_username(username) catch err
    case is ValidationError.EmptyInput
      @println("Username cannot be empty")
      return
    case is ValidationError.TooShort
      @println(`Username too short: ${err.actual} chars (min: ${err.min})`)
      return
    case is ValidationError.TooLong
      @println(`Username too long: ${err.actual} chars (max: ${err.max})`)
      return
    case _
      @println("Invalid username")
      return
  end

  @println(`Account created: ${username}`)
end

create_account("alice")
create_account("ab")  # Too short
```

---

## Tips and Best Practices

### Organize Code with Functions

Break large scripts into smaller functions:

```tea
def read_input() -> String
  # Input logic
end

def process_data(data: String) -> String
  # Processing logic
end

def write_output(data: String)
  # Output logic
end

def main()
  var input = read_input()
  var processed = process_data(input)
  write_output(processed)
end

main()
```

### Use Structs for Related Data

Group related values into structs:

```tea
# Instead of this:
def process(name: String, age: Int, email: String)
  # ...
end

# Use this:
struct User {
  name: String
  age: Int
  email: String
}

def process_user(user: User)
  # ...
end
```

### Validate Early

Check preconditions at the start of functions:

```tea
def divide(a: Float, b: Float) -> Float
  if b == 0.0
    @println("Error: division by zero")
    return 0.0
  end

  a / b
end

def process_list(items: List[String])
  if @len(items) == 0
    @println("Warning: empty list")
    return
  end

  # Process items...
end
```

### Use Constants for Configuration

```tea
const MAX_RETRIES = 3
const TIMEOUT_SECONDS = 30
const DEFAULT_PORT = "8080"

def connect_with_retry(host: String)
  var attempts = 0

  while attempts < MAX_RETRIES
    @println(`Attempt ${attempts + 1}`)
    attempts = attempts + 1
  end
end
```

## See Also

- **[Language Guide](guide/basics.md)** - Learn Tea fundamentals
- **[Standard Library](reference/standard-library.md)** - Complete stdlib reference
- **[Built-in Functions](reference/builtins.md)** - Global functions reference
- **[Project Examples](../examples/)** - More example code in the repository
