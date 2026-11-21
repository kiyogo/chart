// Third-party ChatW Plugin Worker: acme.task_tracker
// Uses ONLY the public sandboxed `chatw.*` SDK — zero ChatW core edits.

async function loadTasks() {
  const existing = await chatw.storage.get('tasks');
  return Array.isArray(existing) ? existing : [];
}

async function saveTasks(tasks) {
  await chatw.storage.set('tasks', tasks, { syncable: true });
}

chatw.onReady(async () => {
  await chatw.ui.registerMessageAction({
    id: 'create_task_card',
    label: 'Tạo thẻ',
    icon: 'check-square',
  });

  await chatw.ui.registerComposerButton({
    id: 'insert_task_summary',
    label: 'Chèn danh sách việc',
    content: '/tasks',
  });

  await chatw.commands.register('/track', 'Tạo nhanh thẻ công việc mới', async (ctx) => {
    const title = String(ctx.args || '').trim();
    if (!title) {
      return { text: 'Cú pháp: /track <tên công việc>' };
    }
    const settings = await chatw.settings.get();
    const col = (settings && settings.default_column) || 'todo';
    const prefix = (settings && settings.auto_tag_prefix) || '[TASK]';
    const tasks = await loadTasks();
    const item = {
      id: 't_' + Date.now(),
      title: `${prefix} ${title}`.trim(),
      column: col,
      createdAt: new Date().toISOString(),
    };
    tasks.unshift(item);
    await saveTasks(tasks);
    await chatw.ui.notify(`Đã thêm công việc: ${item.title}`, 'success');
    return { text: `Đã tạo thẻ công việc "${item.title}" (${col}). Tổng số thẻ: ${tasks.length}` };
  });

  await chatw.commands.register('/tasks', 'Hiển thị tóm tắt thẻ công việc', async () => {
    const tasks = await loadTasks();
    const open = tasks.filter((t) => t.column !== 'done').length;
    return { text: `Task Tracker Pro: ${open} việc đang mở / ${tasks.length} tổng số thẻ.` };
  });

  chatw.onMessageAction('create_task_card', async (message) => {
    const snippet = String(message.content || message.text || 'Tin nhắn không có văn bản').slice(0, 120);
    const tasks = await loadTasks();
    const item = {
      id: 'm_' + Date.now(),
      title: `[MSG] ${snippet}`,
      column: 'todo',
      sourceMessageId: message.id || null,
      createdAt: new Date().toISOString(),
    };
    tasks.unshift(item);
    await saveTasks(tasks);
    await chatw.ui.notify(`Đã chuyển tin nhắn thành thẻ công việc`, 'success');
    return { ok: true, taskId: item.id, count: tasks.length };
  });

  await chatw.events.on('onMessageReceived', async (payload) => {
    const stats = (await chatw.storage.get('event_stats')) || { receivedCount: 0, lastSnippet: '' };
    stats.receivedCount = Number(stats.receivedCount || 0) + 1;
    stats.lastSnippet = String(payload.content || payload.text || '').slice(0, 80);
    stats.lastAt = new Date().toISOString();
    await chatw.storage.set('event_stats', stats, { syncable: false });
  });
});
