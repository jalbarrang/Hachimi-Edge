#!/usr/bin/env node
// Generate the training-tracker skill-evaluation resource.
//
// Joins two community/extracted sources into a slim, runtime-loadable JSON:
//   1. gradeValue (+ rarity, name) per skill id — authoritative, extracted from
//      the game's master.mdb (uma-sim skills.json export).
//   2. affinity_role per skill — UmaTools' annotation of which aptitude a skill's
//      evaluation scales with (uma_skills.csv).
//
// Output: { "<skillId>": { "g": <gradeValue>, "r": "<role|a/b>", "u": 1 } }
//   g = grade value (base evaluation points)
//   r = aptitude role key (lowercased; "a/b" for compound), omitted when none
//   u = 1 when the skill is a trainee unique (rarity >= 3), omitted otherwise
//
// Usage:
//   node scripts/gen-skill-grades.mjs <skills.json> <uma_skills.csv> <out.json>
// Provenance:
//   skills.json  : D:/work/dreki/uma-sim/src/modules/data/json/skills.json
//   uma_skills.csv: https://raw.githubusercontent.com/daftuyda/UmaTools/main/assets/uma_skills.csv

import { readFileSync, writeFileSync } from 'node:fs';

const [, , skillsPath, csvPath, outPath] = process.argv;
if (!skillsPath || !csvPath || !outPath) {
  console.error('usage: gen-skill-grades.mjs <skills.json> <uma_skills.csv> <out.json>');
  process.exit(1);
}

function parseCSV(t) {
  const rows = [];
  let i = 0,
    f = '',
    row = [],
    q = false;
  while (i < t.length) {
    const c = t[i];
    if (q) {
      if (c === '"') {
        if (t[i + 1] === '"') {
          f += '"';
          i++;
        } else q = false;
      } else f += c;
    } else if (c === '"') q = true;
    else if (c === ',') {
      row.push(f);
      f = '';
    } else if (c === '\n') {
      row.push(f);
      rows.push(row);
      row = [];
      f = '';
    } else if (c !== '\r') f += c;
    i++;
  }
  if (f.length || row.length) {
    row.push(f);
    rows.push(row);
  }
  return rows;
}

const skills = JSON.parse(readFileSync(skillsPath, 'utf8'));
const rows = parseCSV(readFileSync(csvPath, 'utf8'));
const hdr = rows[0];
const nameIdx = hdr.indexOf('name');
const aliasIdx = hdr.indexOf('alias_name');
const locIdx = hdr.indexOf('localized_name');
const roleIdx = hdr.indexOf('affinity_role');

// name -> role (try all name columns)
const roleByName = new Map();
for (const r of rows.slice(1)) {
  const role = (r[roleIdx] || '').trim();
  if (!role) continue;
  for (const idx of [nameIdx, aliasIdx, locIdx]) {
    const n = (r[idx] || '').trim().toLowerCase();
    if (n) roleByName.set(n, role.toLowerCase());
  }
}

const out = {};
let withRole = 0,
    uniques = 0;
for (const [id, s] of Object.entries(skills)) {
  const g = s.gradeValue;
  if (typeof g !== 'number') continue;
  const entry = { g };
  const role = s.name ? roleByName.get(s.name.trim().toLowerCase()) : undefined;
  if (role) {
    entry.r = role;
    withRole++;
  }
  if ((s.rarity || 0) >= 3) {
    entry.u = 1;
    uniques++;
  }
  out[id] = entry;
}

writeFileSync(outPath, JSON.stringify(out));
console.error(`wrote ${Object.keys(out).length} skills (${withRole} with role, ${uniques} unique) -> ${outPath}`);
