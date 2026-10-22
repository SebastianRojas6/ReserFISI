// Mejora progresiva: la navegación y todos los formularios funcionan sin JS.
const dialog = document.querySelector('[role="dialog"]');
if (dialog) {
  document.body.classList.add('modal-open');
  for (const element of document.querySelectorAll('body > header, body > main, body > footer, .skip-link')) element.inert = true;
  const close = dialog.querySelector('.modal-close');
  const focusable = () => [...dialog.querySelectorAll('a[href], button, input:not([type="hidden"]), select')].filter(el => !el.disabled);
  (dialog.querySelector('input:not([type="hidden"])') || dialog).focus();
  document.addEventListener('keydown', event => {
    if (event.key === 'Escape') close.click();
    if (event.key !== 'Tab') return;
    const elements = focusable();
    const first = elements[0];
    const last = elements.at(-1);
    if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
    else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
  });
  dialog.parentElement.addEventListener('click', event => {
    if (event.target === dialog.parentElement) close.click();
  });
}

const schedule = document.querySelector('.schedule-scroll[data-today="true"]');
if (schedule) {
  const hour = Number(new Intl.DateTimeFormat('en-GB', { timeZone: 'America/Lima', hour: '2-digit', hourCycle: 'h23' }).format(new Date()));
  const rows = [...schedule.querySelectorAll('tbody tr')];
  const row = rows.find(row => Number(row.querySelector('th').textContent.slice(0, 2)) >= hour);
  if (row) schedule.scrollTop = row.offsetTop - rows[0].offsetTop;
}

const start = document.querySelector('#start');
const end = document.querySelector('#end');
if (start && end) {
  const minutes = value => Number(value.slice(0, 2)) * 60 + Number(value.slice(3));
  const updateEnd = () => {
    for (const option of end.options) {
      const difference = minutes(option.value) - minutes(start.value);
      option.disabled = difference < 30 || difference > 180;
    }
    if (end.selectedOptions[0]?.disabled) {
      end.value = [...end.options].find(option => !option.disabled)?.value || '';
    }
  };
  start.addEventListener('change', updateEnd);
  updateEnd();
}

for (const form of document.querySelectorAll('form[method="post"]')) {
  form.addEventListener('submit', event => {
    if (form.dataset.submitting) { event.preventDefault(); return; }
    if (form.dataset.confirm && !window.confirm(form.dataset.confirm)) { event.preventDefault(); return; }
    form.dataset.submitting = 'true';
    const button = form.querySelector('button[type="submit"]');
    if (button) { button.disabled = true; button.textContent = 'Un momento…'; }
    form.setAttribute('aria-busy', 'true');
  });
}
// Restaura los formularios si el navegador recupera la página de su caché.
window.addEventListener('pageshow', event => { if (event.persisted) window.location.reload(); });
