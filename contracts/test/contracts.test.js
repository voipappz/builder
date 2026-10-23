import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { validate, SCHEMA_NAMES } from '../src/index.js';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const FIXTURES = join(ROOT, 'fixtures');
const read = (p) => JSON.parse(readFileSync(p, 'utf8'));
const list = (dir) => (existsSync(dir) ? readdirSync(dir).filter((f) => f.endsWith('.json')).sort() : []);

describe('fixtures', () => {
  for (const name of SCHEMA_NAMES) {
    test(`${name} has valid and invalid fixtures`, () => {
      assert.ok(list(join(FIXTURES, name, 'valid')).length > 0, 'no valid fixtures');
      assert.ok(list(join(FIXTURES, name, 'invalid')).length > 0, 'no invalid fixtures');
    });
    for (const file of list(join(FIXTURES, name, 'valid'))) {
      test(`${name} accepts ${file}`, () => {
        const result = validate(name, read(join(FIXTURES, name, 'valid', file)));
        assert.equal(result.valid, true, JSON.stringify(result.errors));
      });
    }
    for (const file of list(join(FIXTURES, name, 'invalid'))) {
      test(`${name} rejects ${file} at its stated path`, () => {
        const expected = read(join(FIXTURES, name, 'expected-errors.json'))[file];
        assert.ok(expected, `no expected path recorded for ${file}`);
        const result = validate(name, read(join(FIXTURES, name, 'invalid', file)));
        assert.equal(result.valid, false);
        assert.ok(result.errors.some((e) => e.path === expected), JSON.stringify(result.errors));
      });
    }
  }
});

describe('project descriptor', () => {
  test('the repository example validates once its revision is filled in', () => {
    const example = read(join(ROOT, '..', 'examples', 'builder-project.example.json'));
    assert.equal(validate('project-descriptor', example).valid, true, JSON.stringify(validate('project-descriptor', example).errors));
  });
  test('no property name in any schema suggests a stored secret', () => {
    const forbidden = /token|password|secret|apikey|api_key|authorization|cookie/i;
    for (const file of readdirSync(join(ROOT, 'v1'))) {
      const walk = (node, path) => {
        if (node && typeof node === 'object') {
          for (const [key, value] of Object.entries(node)) {
            if (path.endsWith('properties') && forbidden.test(key)) assert.fail(`${file}: property ${key}`);
            walk(value, `${path}/${key}`);
          }
        }
      };
      walk(JSON.parse(readFileSync(join(ROOT, 'v1', file), 'utf8')), '');
    }
  });
});

test('unknown contract name throws', () => {
  assert.throws(() => validate('nope', {}), /Unknown contract/);
});
