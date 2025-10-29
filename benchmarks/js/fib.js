// Equivalent JavaScript implementation for benchmarking
// Run with: bun fib.js

function fib(n) {
  if (n <= 1) {
    return n;
  }
  return fib(n - 1) + fib(n - 2);
}

function main() {
  const n = 40;
  const result = fib(n);
  console.log(result);
}

main();
