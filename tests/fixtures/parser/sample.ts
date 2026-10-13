import { readFile } from "node:fs";
import Api, { ApiOptions } from "./api";
import * as path from "node:path";

export interface Config {
  root: string;
  limit: number;
}

export type Alias = Config | null;

export enum Mode {
  Fast,
  Slow,
}

export const VERSION = "0.1.0";

export function greet(name: string): string {
  return `hi ${name}`;
}

export function* iterate(items: string[]) {
  for (const item of items) {
    yield item;
  }
}

const helper = async (x: number): Promise<number> => {
  return x + 1;
};

export class Service {
  private cache: Map<string, string>;

  constructor(root: string) {
    this.cache = new Map();
  }

  async run(task: string): Promise<string> {
    return task;
  }
}

export class DefaultService extends Service {}

export { greet as greetAlias };
export * from "./extra";
export default helper;
