console.log('🧪 Testing Heading Hashtag Stripper...');

function cleanHeadingLine(line: string) {
  const trimmed = line.trim();
  const headingMatch = trimmed.match(/^(#{1,6})\s+(.*)$/);
  if (headingMatch) {
    const level = headingMatch[1].length;
    const headingText = headingMatch[2].replace(/#+$/, '').trim();
    return { level, text: headingText };
  }
  return null;
}

const h1 = cleanHeadingLine('# System Architecture Overview');
if (!h1 || h1.level !== 1 || h1.text !== 'System Architecture Overview') {
  throw new Error(`H1 test failed: ${JSON.stringify(h1)}`);
}
console.log('✔ H1 passed: "# System Architecture Overview" -> "System Architecture Overview"');

const h2 = cleanHeadingLine('## **📱 Mobile App Development Skills**');
if (!h2 || h2.level !== 2 || h2.text !== '**📱 Mobile App Development Skills**') {
  throw new Error(`H2 test failed: ${JSON.stringify(h2)}`);
}
console.log('✔ H2 passed: "## **📱 Mobile App Development Skills**" -> "**📱 Mobile App Development Skills**" (no hashtags)');

const h3 = cleanHeadingLine('### Execution Strategy ###');
if (!h3 || h3.level !== 3 || h3.text !== 'Execution Strategy') {
  throw new Error(`H3 test failed: ${JSON.stringify(h3)}`);
}
console.log('✔ H3 passed: "### Execution Strategy ###" -> "Execution Strategy"');

console.log('\n🎉 ALL HEADING HASHTAG STRIPPING TESTS PASSED!');
