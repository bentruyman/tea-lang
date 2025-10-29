// Equivalent JavaScript implementation for benchmarking
// Run with: bun math.js

function compute(n) {
  let result = 0;
  let i = 0;

  while (i < n) {
    result = result + i * 2 - Math.floor(i / 2);
    i += 1;
  }

  return result;
}

function main() {
  const iterations = 1000;
  const n = 10000;

  let i = 0;
  let result = 0;

  while (i < iterations) {
    result = compute(n);
    i += 1;
  }

  console.log(result);
}

main();
