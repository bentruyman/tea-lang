// Equivalent JavaScript implementation for benchmarking
// Run with: bun loops.js

function sumToN(n) {
  let total = 0;
  let i = 1;
  while (i <= n) {
    total = total + i;
    i = i + 1;
  }
  return total;
}

function main() {
  const iterations = 10000;
  const n = 1000;

  let i = 0;
  let result = 0;
  while (i < iterations) {
    result = sumToN(n);
    i = i + 1;
  }

  console.log(result);
}

main();
