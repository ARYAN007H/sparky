import assert from 'node:assert/strict';
import test from 'node:test';
import { declarations, reviewChanges } from '../ci/check-process-statics.mjs';

const file = 'crates/openhuman-core/src/example/ops.rs';
const source = 'static CACHE: OnceLock<Mutex<Map>> = OnceLock::new();';
const entry = {
  file, name: 'CACHE', type: 'OnceLock<Mutex<Map>>', count: 1,
  category: 'B', reason: 'Keys include the authenticated profile.',
  verification: 'example/ops_tests.rs: profiles_cannot_share_cache',
};
const scan = (text) => declarations(text, file);

test('finds multiline, qualified, mutable and lazy_static declarations', () => {
  assert.deepEqual(scan('static CACHE:\n std::sync::OnceLock<Mutex<Map>> = x;\nstatic mut FLAG: bool = false;\nlazy_static! { static ref OTHER: Alias = x; }').map(({name,type}) => ({name,type})), [
    {name:'CACHE',type:'std::sync::OnceLock<Mutex<Map>>'},
    {name:'FLAG',type:'mut bool'}, {name:'OTHER',type:'ref Alias'},
  ]);
});
test('ignores comments and quoted Rust fixtures', () => {
  assert.deepEqual(scan('// '+source+'\n/* outer /* '+source+' */ */\nconst S: &str = r###"'+source+'"###;'), []);
});
test('unreviewed new statics fail even when their type hides a mutex', () => {
  assert.equal(reviewChanges([], scan('static CACHE: Alias = x;'), []).length, 1);
});
test('unchanged existing statics need no retrospective approval', () => {
  assert.deepEqual(reviewChanges(scan(source), scan(source), []), []);
});
test('a new declaration passes with an exact ownership record', () => {
  assert.deepEqual(reviewChanges([], scan(source), [entry]), []);
});
test('type changes and duplicate declarations require a matching record', () => {
  assert.equal(reviewChanges(scan(source), scan(source.replace('Map', 'Other')), [entry]).length > 0, true);
  assert.equal(reviewChanges(scan(source), scan(source+'\nmod nested { '+source+' }'), [entry]).length > 0, true);
});
test('removed or mismatched approvals fail', () => {
  assert.equal(reviewChanges([], [], [entry]).length > 0, true);
  assert.equal(reviewChanges([], scan(source), [{...entry, count:2}]).length > 0, true);
});
test('classification, reason and verification are mandatory', () => {
  for (const patch of [{category:'Z'}, {reason:''}, {verification:''}]) {
    assert.equal(reviewChanges([], scan(source), [{...entry,...patch}]).length > 0, true);
  }
});
test('duplicate approval records fail', () => {
  assert.equal(reviewChanges([], scan(source), [entry,entry]).length > 0, true);
});
