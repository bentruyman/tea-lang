// Equivalent JavaScript implementation for benchmarking
// Run with: bun strings.js

function buildString(count) {
  let result = "";
  let i = 0;

  while (i < count) {
    result = result + "x";
    i += 1;
  }

  return result;
}

function main() {
  const iterations = 100;
  const stringLength = 1000;

  let i = 0;
  let result = "";

  while (i < iterations) {
    result = buildString(stringLength);
    i += 1;
  }

  console.log(`Built string of length ${result.length}`);
}

main();
