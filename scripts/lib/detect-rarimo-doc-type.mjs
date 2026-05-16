#!/usr/bin/env node
/**
 * Detect Rarimo ICAO doc type from register circuit name (process_passport output).
 * registerIdentity_<sig>_<hash>_<docType>_...
 * docType 1 = TD1 (national ID card), 3 = TD3 (passport)
 */
import { readFileSync } from "node:fs";

const name = process.argv[2] || "";
const m = name.match(/registerIdentity_\d+_\d+_(\d+)_/);
const docType = m ? parseInt(m[1], 10) : 3;
const label = docType === 1 ? "td1" : "td3";
console.log(label);
