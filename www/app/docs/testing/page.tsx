import { Card } from "@/components/ui/card"
import Link from "next/link"
import { ArrowRight } from "lucide-react"

export default function TestingPage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Testing</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          Learn how to write and run tests for Tea programs and contribute tests to the Tea compiler.
        </p>
      </div>

      {/* Testing Tea Programs */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Testing Tea Programs</h2>
        <p className="text-muted-foreground">
          Use the <code className="text-accent">std.assert</code> module to write tests for your Tea code.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Basic Assertions</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`use assert = "std.assert"

# Assert a condition is true
assert.ok(1 + 1 == 2)
assert.ok(true)

# Assert two values are equal
assert.eq(1 + 1, 2)
assert.eq("hello", "hello")

# Assert two values are not equal
assert.ne(1, 2)
assert.ne("hello", "world")`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Testing Functions</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`use assert = "std.assert"

def add(a: Int, b: Int) -> Int
  a + b
end

def factorial(n: Int) -> Int
  if n <= 1
    return 1
  end
  n * factorial(n - 1)
end

# Test the functions
assert.eq(add(2, 3), 5)
assert.eq(add(0, 0), 0)
assert.eq(add(-1, 1), 0)

assert.eq(factorial(0), 1)
assert.eq(factorial(1), 1)
assert.eq(factorial(5), 120)`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Testing Structs</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`use assert = "std.assert"

struct Point {
  x: Int
  y: Int
}

def distance_from_origin(p: Point) -> Int
  p.x * p.x + p.y * p.y
end

var origin = Point(x: 0, y: 0)
var point = Point(x: 3, y: 4)

assert.eq(distance_from_origin(origin), 0)
assert.eq(distance_from_origin(point), 25)`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Running Tests */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Running Tests</h2>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Running a Test File</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Run a test file
tea tests/my_test.tea

# If all assertions pass, the program exits successfully
# If any assertion fails, the program panics with an error`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Compiler Tests */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Compiler Tests</h2>
        <p className="text-muted-foreground">
          If you're contributing to the Tea compiler, here's how to run and write tests.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Running All Tests</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Run all tests (Rust tests + E2E tests)
make test

# Run Rust tests only
cargo test --workspace

# Run E2E tests only
./scripts/e2e.sh`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Running Specific Tests</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Run a specific test by name
cargo test -p tea-compiler test_name

# Examples:
cargo test -p tea-compiler interpolated_strings
cargo test -p tea-compiler generics
cargo test -p tea-compiler error_handling`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Test Organization</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Compiler tests are organized by feature in <code className="text-accent">tea-compiler/tests/</code>.
          </p>
          <pre className="font-mono text-sm overflow-x-auto">
            {`tea-compiler/tests/
├── aot_examples.rs     # AOT compilation tests
├── generics.rs         # Generic type tests
├── error_handling.rs   # Error handling tests
├── strings.rs          # String operation tests
└── ...`}
          </pre>
        </Card>
      </div>

      {/* Writing Compiler Tests */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Writing Compiler Tests</h2>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Integration Test Structure</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`// tea-compiler/tests/my_feature.rs

#[test]
fn test_feature_works() {
    let source = r#"
        def add(a: Int, b: Int) -> Int
          a + b
        end

        @println(add(1, 2))
    "#;

    // Compile and run the source
    let result = compile_and_run(source);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().stdout, "3\\n");
}`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Example Tests</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Add example programs to <code className="text-accent">examples/</code> that are automatically tested.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# examples/language/my_feature/test.tea
# Expect: expected output

use assert = "std.assert"

# Your test code here
var result = my_function()
assert.eq(result, expected_value)

@println("expected output")`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Test Best Practices */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Best Practices</h2>

        <Card className="p-6 bg-card border-border">
          <ul className="space-y-3 text-sm">
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                Write tests for each new feature or bug fix
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                Test edge cases and error conditions
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                Keep tests focused and independent
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                Use descriptive test names that explain what's being tested
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                Run <code className="text-accent">make test</code> before submitting PRs
              </span>
            </li>
          </ul>
        </Card>
      </div>

      {/* Next Steps */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Next Steps</h2>

        <div className="flex flex-col gap-4">
          <Link
            href="/docs/contributing"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Contributing Guide</h3>
              <p className="text-sm text-muted-foreground">Submit your tests with a pull request</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/docs/code-style"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Code Style</h3>
              <p className="text-sm text-muted-foreground">Format your test code correctly</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>
        </div>
      </div>
    </div>
  )
}
