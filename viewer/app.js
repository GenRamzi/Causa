const state = { tape: null, events: [] };
const $ = (id) => document.getElementById(id);

function setTape(tape, filename) {
  state.tape = tape;
  state.events = Array.isArray(tape.events) ? tape.events : [];
  $('dropzone').classList.add('hidden');
  $('app').classList.remove('hidden');
  $('summary').innerHTML = [
    ['RUN', tape.metadata?.run_id || 'unknown'],
    ['STEPS', state.events.length],
    ['FORMAT', tape.format || 'unknown'],
    ['MERKLE ROOT', (tape.merkle_root || '').slice(0, 18) + '…']
  ].map(([key, value]) => `<div class="metric"><span>${escapeHtml(key)}</span><strong>${escapeHtml(String(value))}</strong></div>`).join('');
  $('integrity').textContent = `Loaded locally · ${filename || 'tape'}`;
  render();
}

function render() {
  const query = ($('filter').value || '').toLowerCase();
  const events = state.events.filter(e => {
    const labels = (e.labels || []).map(l => `${l.namespace}:${l.value}`).join(' ');
    return [e.name, e.kind, labels, e.hash].join(' ').toLowerCase().includes(query);
  });
  $('count').textContent = `${events.length}/${state.events.length}`;
  $('timeline').innerHTML = events.map(e => {
    const labels = (e.labels || []).map(l => `${l.namespace}:${l.value}`);
    const tone = labels.some(l => l.namespace === 'web' && l.value === 'untrusted') ? 'warning' : '';
    return `<button class="event ${tone}" data-step="${e.step}"><span class="step">${String(e.step).padStart(2, '0')}</span><span class="event-main"><b>${escapeHtml(e.name)}</b><small>${escapeHtml(e.kind)} · ${escapeHtml((e.hash || '').slice(0, 12))}</small></span><span class="labels">${labels.map(l => `<i>${escapeHtml(l)}</i>`).join('')}</span></button>`;
  }).join('') || '<div class="empty">No matching events.</div>';
  document.querySelectorAll('.event').forEach(button => button.addEventListener('click', () => select(Number(button.dataset.step))));
}

function select(step) {
  const event = state.events.find(e => e.step === step);
  if (!event) return;
  $('detail').textContent = JSON.stringify({
    step: event.step, kind: event.kind, name: event.name, hash: event.hash,
    parents: event.parents || [], labels: event.labels || [], input: event.input, output: event.output
  }, null, 2);
  document.querySelectorAll('.event').forEach(b => b.classList.toggle('selected', Number(b.dataset.step) === step));
}

function escapeHtml(value) {
  return value.replace(/[&<>'"]/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;',"'":'&#39;','"':'&quot;'}[c]));
}

async function readFile(file) {
  try { setTape(JSON.parse(await file.text()), file.name); }
  catch (error) { $('integrity').textContent = `Could not open tape: ${error.message}`; $('integrity').className = 'error'; }
}

$('fileInput').addEventListener('change', e => e.target.files[0] && readFile(e.target.files[0]));
$('filter').addEventListener('input', render);
$('dropzone').addEventListener('dragover', e => { e.preventDefault(); $('dropzone').classList.add('drag'); });
$('dropzone').addEventListener('dragleave', () => $('dropzone').classList.remove('drag'));
$('dropzone').addEventListener('drop', e => { e.preventDefault(); $('dropzone').classList.remove('drag'); e.dataTransfer.files[0] && readFile(e.dataTransfer.files[0]); });
