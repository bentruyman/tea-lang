// Equivalent JavaScript implementation for benchmarking
// Run with: bun dicts.js

function buildDict(n) {
  const result = { _: 0 };
  let i = 0;

  while (i < n) {
    result[`key_${i}`] = i;
    i += 1;
  }

  return result;
}

function lookupDict(dict, count) {
  let total = 0;
  let i = 0;

  while (i < count) {
    const key = `key_${i}`;
    total += dict[key] ?? 0;
    i += 1;
  }

  return total;
}

function main() {
  const iterations = 100;
  const dictSize = 500;

  let i = 0;
  let result = 0;

  while (i < iterations) {
    const dict = buildDict(dictSize);
    result = lookupDict(dict, dictSize);
    i += 1;
  }

  console.log(result);
}

main();
