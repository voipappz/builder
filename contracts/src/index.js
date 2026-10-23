// Validators for the Builder v1 contracts. The JSON Schemas in ../v1 are the
// source of truth; the Ruby API vendors the same files and runs the same
// fixtures, so both sides reach the same verdict on the same input.
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

const V1 = join(dirname(fileURLToPath(import.meta.url)), '..', 'v1');
const readJson = (file) => JSON.parse(readFileSync(join(V1, file), 'utf8'));

export const SCHEMA_NAMES = readdirSync(V1)
  .filter((f) => f.endsWith('.schema.json'))
  .map((f) => f.replace('.schema.json', ''))
  .filter((n) => n !== 'common')
  .sort();

// strictTypes and strictRequired misread if/then blocks that only narrow a
// property already typed at the top level. Unknown keywords stay errors.
const ajv = new Ajv2020({ allErrors: true, strict: true, strictTypes: false, strictRequired: false });
addFormats(ajv, ['date-time']);
for (const file of readdirSync(V1).filter((f) => f.endsWith('.schema.json'))) {
  ajv.addSchema(readJson(file));
}
const validators = Object.fromEntries(
  SCHEMA_NAMES.map((name) => [name, ajv.getSchema(`https://schemas.voipappz.io/builder/v1/${name}.schema.json`)])
);

// Rules a single schema cannot state, keyed by schema name.
const SEMANTIC = {
  checkpoint: (value) =>
    value.validation.revision === value.revision
      ? []
      : [{ path: '/validation/revision', message: 'must equal the checkpoint revision' }],
};

// Point at the offending property for additionalProperties, as the Ruby
// validator does, so both sides report the same path.
const errorPath = (e) => {
  const base = e.instancePath || '';
  if (e.keyword === 'additionalProperties') {
    const token = String(e.params.additionalProperty).replace(/~/g, '~0').replace(/\//g, '~1');
    return `${base}/${token}`;
  }
  return base || '/';
};

/**
 * Validate one document. Returns { valid, errors: [{ path, message }] }.
 */
export function validate(name, value) {
  const check = validators[name];
  if (!check) throw new Error(`Unknown contract: ${name}`);
  if (!check(value)) {
    return {
      valid: false,
      errors: check.errors.map((e) => ({ path: errorPath(e), message: e.message })),
    };
  }
  const extra = SEMANTIC[name]?.(value) ?? [];
  return { valid: extra.length === 0, errors: extra };
}
