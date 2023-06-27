import fs from 'fs';

const configFile = '../config.json';
const readmeFile = '../README.md';

// Read config file
const config = JSON.parse(fs.readFileSync(configFile));

// Generate config table
let configTable = '| Key | Type | Description |\n| --- | --- | --- |\n';
for (const [key, value] of Object.entries(config)) {
  const type = Array.isArray(value) ? 'array' : typeof value;
  configTable += `| ${key} | ${type} |  |\n`;
}

console.log(configTable);

/* // Read README file
let readmeContent = fs.readFileSync(readmeFile, 'utf8');

// Replace/insert config table in README
const settingsHeadingRegex = /^## Settings/m;
if (settingsHeadingRegex.test(readmeContent)) {
  readmeContent = readmeContent.replace(/(^## Settings[\s\S]*?\n\n)([\s\S]*?)(?=\n##)/m, `$1${configTable}\n`);
} else {
  readmeContent += `\n\n## Settings\n\n${configTable}\n`;
}

// Write updated README file
fs.writeFileSync(readmeFile, readmeContent);
 */