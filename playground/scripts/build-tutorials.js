#!/usr/bin/env node

/**
 * Build script to convert tutorial markdown files to JSON
 * Reads .md files from tutorials/ directory, extracts frontmatter and content,
 * and generates tutorials.json for the tutorial playground.
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const TUTORIALS_DIR = path.join(__dirname, '../src/tutorials');
const OUTPUT_FILE = path.join(__dirname, '../dist/tutorials.json');

/**
 * Parse frontmatter from markdown content
 * Format:
 * ---
 * title: "Tutorial Title"
 * code: "sample code"
 * ---
 * Markdown content here...
 */
function parseFrontmatter(content) {
  const frontmatterRegex = /^---\n([\s\S]*?)\n---\n([\s\S]*)$/;
  const match = content.match(frontmatterRegex);

  if (!match) {
    return { metadata: {}, content: content.trim() };
  }

  const [, frontmatter, markdown] = match;
  const metadata = {};

  // Parse YAML-style frontmatter
  const lines = frontmatter.split('\n');
  let currentKey = null;
  let currentValue = '';

  for (const line of lines) {
    if (line.trim() === '') continue;

    // Check if this is a key: value line
    const keyMatch = line.match(/^(\w+):\s*(.*)$/);
    if (keyMatch) {
      // Save previous key-value if exists
      if (currentKey) {
        metadata[currentKey] = parseValue(currentValue.trim());
      }

      currentKey = keyMatch[1];
      currentValue = keyMatch[2];
    } else {
      // Continuation of multiline value
      currentValue += '\n' + line;
    }
  }

  // Save last key-value
  if (currentKey) {
    metadata[currentKey] = parseValue(currentValue.trim());
  }

  return { metadata, content: markdown.trim() };
}

/**
 * Parse a value from frontmatter (handle strings, numbers, multiline)
 */
function parseValue(value) {
  // Remove quotes if present
  if ((value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))) {
    return value.slice(1, -1);
  }

  // Handle pipe (|) for multiline strings
  if (value.startsWith('|')) {
    return value.substring(1).trim();
  }

  // Try to parse as number
  const num = Number(value);
  if (!isNaN(num)) {
    return num;
  }

  // Return as string
  return value;
}

/**
 * Read and parse all tutorial markdown files
 */
function buildTutorials() {
  console.log('🖖 Building tutorials...\n');

  // Ensure tutorials directory exists
  if (!fs.existsSync(TUTORIALS_DIR)) {
    console.error(`❌ Tutorials directory not found: ${TUTORIALS_DIR}`);
    process.exit(1);
  }

  // Read all .md files
  const files = fs.readdirSync(TUTORIALS_DIR)
    .filter(file => file.endsWith('.md'))
    .sort(); // Sort alphabetically (01-*, 02-*, etc.)

  if (files.length === 0) {
    console.warn('⚠️  No tutorial files found in tutorials/ directory');
    return [];
  }

  console.log(`Found ${files.length} tutorial files:\n`);

  const tutorials = files.map(file => {
    const filepath = path.join(TUTORIALS_DIR, file);
    const content = fs.readFileSync(filepath, 'utf-8');
    const { metadata, content: markdown } = parseFrontmatter(content);

    console.log(`  ✓ ${file} - "${metadata.title || 'Untitled'}"`);

    return {
      id: path.basename(file, '.md'),
      title: metadata.title || 'Untitled',
      code: metadata.code || '',
      content: markdown,
    };
  });

  console.log(`\n📝 Parsed ${tutorials.length} tutorials`);
  return tutorials;
}

/**
 * Write tutorials to JSON file
 */
function writeTutorials(tutorials) {
  // Ensure dist directory exists
  const distDir = path.dirname(OUTPUT_FILE);
  if (!fs.existsSync(distDir)) {
    fs.mkdirSync(distDir, { recursive: true });
  }

  // Write JSON
  fs.writeFileSync(OUTPUT_FILE, JSON.stringify(tutorials, null, 2), 'utf-8');
  console.log(`\n✅ Generated ${OUTPUT_FILE}`);
  console.log(`   ${tutorials.length} tutorials ready for the playground\n`);
}

/**
 * Main execution
 */
function main() {
  try {
    const tutorials = buildTutorials();
    writeTutorials(tutorials);
  } catch (error) {
    console.error('\n❌ Error building tutorials:', error.message);
    process.exit(1);
  }
}

main();
