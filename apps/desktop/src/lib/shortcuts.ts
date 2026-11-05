export function isShortcutControl(target: EventTarget | null): boolean {
  return (
    target instanceof Element &&
    Boolean(
      target.closest(
        'button, a[href], summary, input, textarea, select, [contenteditable], [role="button"], [role="tab"], [role="dialog"], dialog',
      ),
    )
  );
}
