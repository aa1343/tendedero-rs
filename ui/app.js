// Tendedero (Rust Lightweight Edition) Frontend Application
const invoke = async (cmd, args) => {
  try {
    if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
      return await window.__TAURI__.core.invoke(cmd, args);
    }
  } catch (e) {
    console.warn(`invoke [${cmd}] error:`, e);
  }
  return [];
};

const listen = (event, handler) => {
  if (window.__TAURI__ && window.__TAURI__.event && window.__TAURI__.event.listen) {
    return window.__TAURI__.event.listen(event, handler);
  }
  return Promise.resolve(() => {});
};

let cards = [];
let isRevealed = true;
let currentModalTodoId = null;

const lineRoot = document.getElementById('lineRoot');
const cardsContainer = document.getElementById('cards');
const ropeSvg = document.getElementById('rope');
const hintEl = document.getElementById('hint');
const toastEl = document.getElementById('toast');
const modalOverlay = document.getElementById('modalOverlay');
const searchModal = document.getElementById('searchModal');
const searchInput = document.getElementById('searchInput');
const searchResults = document.getElementById('searchResults');
const todoDetailModal = document.getElementById('todoDetailModal');
const modalTodoTitle = document.getElementById('modalTodoTitle');
const modalTodoList = document.getElementById('modalTodoList');
const modalNewItemInput = document.getElementById('modalNewItemInput');
const modalTodoStats = document.getElementById('modalTodoStats');

// ---------------------------------------------------------------------------
// Toast helper
// ---------------------------------------------------------------------------
let toastTimer = null;
function showToast(msg) {
  toastEl.textContent = msg;
  toastEl.classList.add('show');
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toastEl.classList.remove('show');
  }, 2200);
}

// ---------------------------------------------------------------------------
// Mouse Pass-Through is managed safely & centrally by the Rust backend.
// ---------------------------------------------------------------------------
function attachHoverDetection() {
  // Backend automatically handles pass-through:
  // - Fully passes through when tucked in
  // - Interactive when revealed
}

// ---------------------------------------------------------------------------
// Clothesline Rope Drawing & Geometry
// ---------------------------------------------------------------------------
function drawRope() {
  const w = window.innerWidth;
  const sag = Math.min(26, w * 0.016);
  ropeSvg.setAttribute('width', w);
  ropeSvg.setAttribute('height', 60);

  // Smooth quadratic catenary bezier
  const pathD = `M 0 10 Q ${w / 2} ${10 + sag * 2} ${w} 10`;
  ropeSvg.innerHTML = `
    <path d="${pathD}" stroke="rgba(0,0,0,0.45)" stroke-width="1.8" fill="none" stroke-linecap="round"/>
    <path d="${pathD}" stroke="rgba(255,255,255,0.35)" stroke-width="0.8" fill="none" stroke-linecap="round" transform="translate(0, 0.6)"/>
  `;
}

function getRopeY(x) {
  const w = window.innerWidth;
  if (w <= 0) return 10;
  const fx = Math.min(1, Math.max(0, x / w));
  const sag = Math.min(26, w * 0.016);
  return 10 + 4 * sag * fx * (1 - fx);
}

// ---------------------------------------------------------------------------
// Render Cards
// ---------------------------------------------------------------------------
function renderCards() {
  cardsContainer.innerHTML = '';

  if (cards.length === 0) {
    hintEl.style.display = 'block';
    return;
  }
  hintEl.style.display = 'none';

  const cardWidth = 175;
  const spacing = 195;
  const startX = 40;

  cards.forEach((card, index) => {
    const cardX = startX + index * spacing;
    const ropeY = getRopeY(cardX + cardWidth / 2);

    const cardEl = document.createElement('div');
    cardEl.className = `card ${card.pinned ? 'pinned' : ''}`;
    cardEl.id = `card-${card.id}`;
    cardEl.style.left = `${cardX}px`;
    cardEl.style.top = `${ropeY - 14}px`;

    // Clothespin (木夹子)
    const clip = document.createElement('div');
    clip.className = 'clip';
    clip.innerHTML = '<div class="slot"></div>';
    cardEl.appendChild(clip);

    // Hit box
    const hit = document.createElement('div');
    hit.className = 'hit';

    // Corner buttons (Pin, Delete)
    const delBtn = document.createElement('button');
    delBtn.className = 'corner-btn corner-del';
    delBtn.title = '取下卡片 (Del)';
    delBtn.innerHTML = '<svg viewBox="0 0 24 24"><path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/></svg>';
    delBtn.onclick = (e) => {
      e.stopPropagation();
      removeCard(card.id);
    };

    const pinBtn = document.createElement('button');
    pinBtn.className = 'corner-btn corner-pin';
    pinBtn.title = card.pinned ? '取消固定' : '固定在绳上';
    pinBtn.innerHTML = '<svg viewBox="0 0 24 24"><path d="M16 12V4h1V2H7v2h1v8l-2 2v2h5.2v6h1.6v-6H18v-2l-2-2z"/></svg>';
    pinBtn.onclick = (e) => {
      e.stopPropagation();
      togglePin(card.id);
    };

    // Glass body
    const glass = document.createElement('div');
    glass.className = 'glass';
    glass.appendChild(delBtn);
    glass.appendChild(pinBtn);

    // Card specifics
    if (card.cardType === 'todo' && card.todoData) {
      renderTodoCardContent(glass, card);
      cardEl.classList.add('todo-card', card.todoData.color || 'yellow');
    } else if (card.cardType === 'screenshot' || card.cardType === 'image') {
      renderImageCardContent(glass, card);
    } else {
      renderNoteCardContent(glass, card);
    }

    hit.appendChild(glass);
    cardEl.appendChild(hit);

    // Card interactions
    hit.onclick = (e) => {
      if (e.target.closest('button') || e.target.closest('.todo-check') || e.target.closest('input')) return;
      copyCard(card.id);
    };

    hit.ondblclick = (e) => {
      if (card.cardType === 'todo') {
        openTodoDetailModal(card.id);
      } else {
        invoke('open_card', { id: card.id });
      }
    };

    cardsContainer.appendChild(cardEl);
  });

  attachHoverDetection();
}

function renderImageCardContent(glass, card) {
  const photo = document.createElement('div');
  photo.className = 'photo-wrap';
  const img = document.createElement('img');
  img.src = card.thumbnailData || card.content;
  photo.appendChild(img);

  const info = document.createElement('div');
  info.className = 'card-info';
  info.innerHTML = `
    <div class="card-title">${card.title}</div>
    <div class="card-meta">${card.meta || '图片'}</div>
  `;

  glass.appendChild(photo);
  glass.appendChild(info);
}

function renderNoteCardContent(glass, card) {
  const sheet = document.createElement('div');
  sheet.className = 'note-sheet';

  if (card.cardType === 'link') {
    sheet.innerHTML = `
      <div>
        <div class="link-badge">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M3.9 12c0-1.71 1.39-3.1 3.1-3.1h4V7H7c-2.76 0-5 2.24-5 5s2.24 5 5 5h4v-1.9H7c-1.71 0-3.1-1.39-3.1-3.1zM8 13h8v-2H8v2zm9-6h-4v1.9h4c1.71 0 3.1 1.39 3.1 3.1s-1.39 3.1-3.1 3.1h-4V17h4c2.76 0 5-2.24 5-5s-2.24-5-5-5z"/></svg>
          <span>${card.title}</span>
        </div>
        <div class="note-text">${card.content}</div>
      </div>
      <div class="card-meta">${card.meta || '网页链接'}</div>
    `;
  } else {
    sheet.innerHTML = `
      <div>
        <div class="card-title" style="margin-bottom: 6px;">${card.title}</div>
        <div class="note-text">${card.content}</div>
      </div>
      <div class="card-meta">${card.meta || '便签'}</div>
    `;
  }
  glass.appendChild(sheet);
}

// ---------------------------------------------------------------------------
// TODO Sticky Note Rendering & Interactions
// ---------------------------------------------------------------------------
function renderTodoCardContent(glass, card) {
  const todo = card.todoData;
  const header = document.createElement('div');
  header.className = 'todo-header';
  header.innerHTML = `
    <div class="todo-title" title="${todo.title}">${todo.title}</div>
  `;

  const list = document.createElement('div');
  list.className = 'todo-list';

  todo.items.forEach(item => {
    const row = document.createElement('div');
    row.className = `todo-item-row ${item.done ? 'done' : ''}`;

    const check = document.createElement('div');
    check.className = `todo-check ${item.done ? 'done' : ''}`;
    check.onclick = (e) => {
      e.stopPropagation();
      toggleTodoItem(card.id, item.id, !item.done);
    };

    const text = document.createElement('span');
    text.className = 'todo-item-text';
    text.textContent = item.text;

    row.appendChild(check);
    row.appendChild(text);
    list.appendChild(row);
  });

  // Quick inline add input
  const quickAdd = document.createElement('div');
  quickAdd.className = 'todo-quick-add';
  const input = document.createElement('input');
  input.type = 'text';
  input.className = 'todo-input-field';
  input.placeholder = '+ 回车快速添加...';
  input.onclick = (e) => e.stopPropagation();
  input.onkeydown = (e) => {
    if (e.key === 'Enter' && input.value.trim()) {
      addTodoItem(card.id, input.value.trim());
      input.value = '';
    }
  };
  quickAdd.appendChild(input);

  // Progress Bar
  const total = todo.items.length;
  const done = todo.items.filter(i => i.done).count ? todo.items.filter(i => i.done).length : todo.items.filter(i => i.done).length;
  const pct = total === 0 ? 0 : Math.round((done / total) * 100);

  const prog = document.createElement('div');
  prog.className = 'todo-progress';
  prog.innerHTML = `
    <div class="progress-track"><div class="progress-fill" style="width: ${pct}%"></div></div>
    <div style="font-size: 9.5px; color: #666; margin-top: 3px; display: flex; justify-content: space-between;">
      <span>${done}/${total} 已完成</span>
      <span>${pct}%</span>
    </div>
  `;

  glass.appendChild(header);
  glass.appendChild(list);
  glass.appendChild(quickAdd);
  glass.appendChild(prog);
}

// ---------------------------------------------------------------------------
// Card & TODO Operations
// ---------------------------------------------------------------------------
async function loadCards() {
  try {
    cards = await invoke('get_cards');
    renderCards();
  } catch (e) {
    console.error('Failed to load cards:', e);
  }
}

async function removeCard(id) {
  try {
    await invoke('remove_card', { id });
    cards = cards.filter(c => c.id !== id);
    renderCards();
    showToast('已从绳上取下 (按 Ctrl+Z 撤销)');
  } catch (e) {
    console.error(e);
  }
}

async function undoRemove() {
  try {
    const card = await invoke('undo_remove');
    if (card) {
      cards.unshift(card);
      renderCards();
      showToast('已撤销取下卡片！');
    }
  } catch (e) {
    console.error(e);
  }
}

async function togglePin(id) {
  try {
    const pinned = await invoke('toggle_pin', { id });
    const card = cards.find(c => c.id === id);
    if (card && pinned !== null) {
      card.pinned = pinned;
      renderCards();
      showToast(pinned ? '已固定卡片 📌' : '已取消固定');
    }
  } catch (e) {
    console.error(e);
  }
}

async function copyCard(id) {
  try {
    const res = await invoke('copy_card_content', { id });
    showToast(res || '已复制到剪贴板！');
  } catch (e) {
    showToast('复制失败: ' + e);
  }
}

async function toggleTodoItem(cardId, itemId, done) {
  try {
    const updated = await invoke('update_todo_item', { cardId, itemId, done });
    if (updated) {
      const idx = cards.findIndex(c => c.id === cardId);
      if (idx !== -1) cards[idx] = updated;
      renderCards();
      if (currentModalTodoId === cardId) {
        refreshModalTodoList(updated.todoData);
      }
    }
  } catch (e) {
    console.error(e);
  }
}

async function addTodoItem(cardId, text) {
  try {
    const updated = await invoke('add_todo_item', { cardId, text });
    if (updated) {
      const idx = cards.findIndex(c => c.id === cardId);
      if (idx !== -1) cards[idx] = updated;
      renderCards();
      if (currentModalTodoId === cardId) {
        refreshModalTodoList(updated.todoData);
      }
    }
  } catch (e) {
    console.error(e);
  }
}

// ---------------------------------------------------------------------------
// Reveal & Tuck Logic
// ---------------------------------------------------------------------------
function setRevealed(revealed) {
  isRevealed = revealed;
  document.body.classList.toggle('revealed', revealed);
  if (revealed) {
    lineRoot.classList.remove('tucked');
  } else {
    lineRoot.classList.add('tucked');
    closeAllModals();
  }
}

// ---------------------------------------------------------------------------
// Modals: Search & Todo Detail
// ---------------------------------------------------------------------------
function closeAllModals() {
  modalOverlay.style.display = 'none';
  searchModal.style.display = 'none';
  todoDetailModal.style.display = 'none';
  invoke('set_locked', { locked: false });
}

function openSearchModal() {
  setRevealed(true);
  modalOverlay.style.display = 'flex';
  searchModal.style.display = 'block';
  todoDetailModal.style.display = 'none';
  searchInput.value = '';
  renderSearchResults('');
  invoke('set_locked', { locked: true });
  setTimeout(() => searchInput.focus(), 50);
}

function renderSearchResults(query) {
  searchResults.innerHTML = '';
  const q = query.trim().toLowerCase();
  const matched = cards.filter(c => {
    if (!q) return true;
    if (c.title.toLowerCase().includes(q)) return true;
    if (c.content.toLowerCase().includes(q)) return true;
    if (c.todoData) {
      return c.todoData.items.some(i => i.text.toLowerCase().includes(q));
    }
    return false;
  });

  if (matched.length === 0) {
    searchResults.innerHTML = '<div style="padding: 16px; text-align: center; color: #8e8e93; font-size: 13px;">无匹配结果</div>';
    return;
  }

  matched.forEach(c => {
    const item = document.createElement('div');
    item.className = 'search-item';
    item.innerHTML = `
      <div>
        <div class="search-item-title">${c.title}</div>
        <div class="search-item-meta">${c.meta || c.cardType}</div>
      </div>
    `;
    item.onclick = () => {
      closeAllModals();
      copyCard(c.id);
    };
    searchResults.appendChild(item);
  });
}

function openTodoDetailModal(cardId) {
  const card = cards.find(c => c.id === cardId);
  if (!card || !card.todoData) return;

  currentModalTodoId = cardId;
  const todo = card.todoData;

  modalTodoTitle.value = todo.title;
  refreshModalTodoList(todo);

  // Active color dot
  document.querySelectorAll('#modalColorPicker .color-dot').forEach(dot => {
    dot.classList.toggle('active', dot.dataset.color === todo.color);
  });

  modalOverlay.style.display = 'flex';
  todoDetailModal.style.display = 'block';
  searchModal.style.display = 'none';
  invoke('set_locked', { locked: true });
}

function refreshModalTodoList(todo) {
  modalTodoList.innerHTML = '';
  const total = todo.items.length;
  const done = todo.items.filter(i => i.done).length;
  modalTodoStats.textContent = `${done}/${total} 已完成`;

  todo.items.forEach(item => {
    const row = document.createElement('div');
    row.className = `todo-item-row ${item.done ? 'done' : ''}`;
    row.style.padding = '4px 0';

    const check = document.createElement('div');
    check.className = `todo-check ${item.done ? 'done' : ''}`;
    check.onclick = () => toggleTodoItem(currentModalTodoId, item.id, !item.done);

    const text = document.createElement('span');
    text.className = 'todo-item-text';
    text.textContent = item.text;

    const del = document.createElement('button');
    del.className = 'modal-close-btn';
    del.innerHTML = '&times;';
    del.style.fontSize = '16px';
    del.onclick = async () => {
      const updated = await invoke('delete_todo_item', { cardId: currentModalTodoId, itemId: item.id });
      if (updated) {
        const idx = cards.findIndex(c => c.id === currentModalTodoId);
        if (idx !== -1) cards[idx] = updated;
        renderCards();
        refreshModalTodoList(updated.todoData);
      }
    };

    row.appendChild(check);
    row.appendChild(text);
    row.appendChild(del);
    modalTodoList.appendChild(row);
  });
}

async function createNewTodoCard() {
  try {
    const card = await invoke('create_todo', {
      title: '🎯 新建待办便签',
      color: 'yellow',
      items: ['双击编辑待办清单', '点击方框勾选完成'],
    });
    cards.unshift(card);
    renderCards();
    setRevealed(true);
    showToast('已挂上新待办便签！');
    openTodoDetailModal(card.id);
  } catch (e) {
    console.error(e);
  }
}

// ---------------------------------------------------------------------------
// Event Listeners & IPC
// ---------------------------------------------------------------------------
document.getElementById('btnNewTodo').onclick = () => createNewTodoCard();
document.getElementById('btnCapture').onclick = async () => {
  try {
    await invoke('capture_screen_now');
  } catch (e) {
    showToast('截图失败: ' + e);
  }
};
document.getElementById('btnSearch').onclick = () => openSearchModal();
document.getElementById('btnDataFolder').onclick = () => invoke('open_data_folder');

document.getElementById('btnCloseModal').onclick = () => closeAllModals();
document.getElementById('btnSaveTodoModal').onclick = () => closeAllModals();

modalTodoTitle.onchange = async () => {
  if (currentModalTodoId && modalTodoTitle.value.trim()) {
    const updated = await invoke('update_todo_title', {
      cardId: currentModalTodoId,
      title: modalTodoTitle.value.trim(),
    });
    if (updated) {
      const idx = cards.findIndex(c => c.id === currentModalTodoId);
      if (idx !== -1) cards[idx] = updated;
      renderCards();
    }
  }
};

modalNewItemInput.onkeydown = async (e) => {
  if (e.key === 'Enter' && modalNewItemInput.value.trim() && currentModalTodoId) {
    await addTodoItem(currentModalTodoId, modalNewItemInput.value.trim());
    modalNewItemInput.value = '';
  }
};

document.querySelectorAll('#modalColorPicker .color-dot').forEach(dot => {
  dot.onclick = async () => {
    if (!currentModalTodoId) return;
    const color = dot.dataset.color;
    document.querySelectorAll('#modalColorPicker .color-dot').forEach(d => d.classList.remove('active'));
    dot.classList.add('active');
    const updated = await invoke('update_todo_color', { cardId: currentModalTodoId, color });
    if (updated) {
      const idx = cards.findIndex(c => c.id === currentModalTodoId);
      if (idx !== -1) cards[idx] = updated;
      renderCards();
    }
  };
});

searchInput.oninput = (e) => renderSearchResults(e.target.value);

window.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') {
    if (modalOverlay.style.display !== 'none') {
      closeAllModals();
    } else if (isRevealed) {
      invoke('set_revealed', { revealed: false });
    }
  }
  if (e.ctrlKey && e.key.toLowerCase() === 'z') {
    undoRemove();
  }
});

// 点击空白区域自动收回晾衣绳并恢复穿透
document.addEventListener('click', (e) => {
  if (modalOverlay.style.display !== 'none') return;
  if (!e.target.closest('.card, .top-btn, .modal-box, .color-dot')) {
    if (isRevealed) {
      invoke('set_revealed', { revealed: false });
    }
  }
});

window.addEventListener('resize', () => {
  drawRope();
  renderCards();
});

// Tauri Event Bus Listeners
listen('set_revealed', (event) => setRevealed(event.payload));
listen('card_added', (event) => {
  cards.unshift(event.payload);
  renderCards();
  setRevealed(true);
  showToast('新内容已晾上绳子！');
});
listen('open_new_todo', () => createNewTodoCard());
listen('open_search', () => openSearchModal());
listen('trigger_capture', async () => {
  try {
    await invoke('capture_screen_now');
  } catch (e) {
    showToast('截图失败: ' + e);
  }
});
listen('open_data_folder', () => invoke('open_data_folder'));
listen('show_toast', (event) => {
  if (event && event.payload) {
    showToast(event.payload);
  }
});

// Initial Boot: 启动时以展开状态呈现，用户一眼可见并可直接交互
window.addEventListener('DOMContentLoaded', () => {
  drawRope();
  loadCards();
  attachHoverDetection();
  setRevealed(true);
  showToast('晾衣绳已就绪！点击空白处或向下移开鼠标即可收起');
});
