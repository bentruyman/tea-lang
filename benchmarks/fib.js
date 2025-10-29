// Benchmark: Recursive fibonacci
// Tests function call overhead and recursion performance

function fib(n) {
  if (n <= 1) {
    return n;
  }
  return fib(n - 1) + fib(n - 2);
}

const n = 35;
const result = fib(n);
console.log(result);
