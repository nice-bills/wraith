#!/usr/bin/env node
import { analyzeDocumentFile } from "./lib/document-path.mjs";

const path = process.argv[2];
if (!path) {
  console.error("Usage: detect-document-path.mjs <document.json>");
  process.exit(1);
}

const result = analyzeDocumentFile(path);
console.log(JSON.stringify(result, null, 2));
