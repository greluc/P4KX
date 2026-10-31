// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/**
 * Validates every `.glb` in the folder given as the first argument with the Khronos
 * glTF-Validator, prints every message and exits with 1 on any error or when no GLB is found.
 */

import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import validator from "gltf-validator";

const SEVERITY = ["error", "warning", "information", "hint"];

const folder = process.argv[2];
if (!folder) {
  console.error("usage: node validate.mjs <folder with .glb files>");
  process.exit(2);
}

const names = (await readdir(folder)).filter((name) => name.endsWith(".glb")).sort();
if (names.length === 0) {
  console.error(`no .glb file in ${folder}`);
  process.exit(1);
}

let errors = 0;
for (const name of names) {
  const bytes = new Uint8Array(await readFile(join(folder, name)));
  const report = await validator.validateBytes(bytes, { uri: name, maxIssues: 0 });
  const { numErrors, numWarnings } = report.issues;
  console.log(`${name}: ${numErrors} errors, ${numWarnings} warnings`);
  for (const message of report.issues.messages) {
    if (message.severity <= 1) {
      console.log(`  ${SEVERITY[message.severity]} ${message.code} ${message.pointer}: ${message.message}`);
    }
  }
  errors += numErrors;
}
process.exit(errors === 0 ? 0 : 1);
