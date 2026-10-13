import { join } from "node:path";
import settings from "./settings.js";

export function add(a, b) {
  return a + b;
}

const multiply = (a, b) => a * b;

export class Counter {
  constructor() {
    this.value = 0;
  }

  increment() {
    this.value += 1;
  }
}

export default add;
export const answer = 42;
