import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  MAX_TOASTS,
  countSuffix,
  dismissToast,
  pushToast,
  type ToastLike,
} from './toast-queue.ts';

let nextId = 1;
function toast(kind: string, title: string, body = 'b', otp: string | null = null): ToastLike {
  return { id: nextId++, kind, title, body, otp };
}

describe('pushToast', () => {
  it('appends a distinct notice', () => {
    const a = toast('Success', 'Detect complete');
    const b = toast('Warning', 'Port lost');
    const list = pushToast(pushToast([], a), b);
    assert.deepEqual(
      list.map((t) => t.title),
      ['Detect complete', 'Port lost']
    );
  });

  it('caps the column at MAX_TOASTS, keeping the newest', () => {
    // Distinct titles so nothing coalesces — this is the cap on its own.
    let list: ToastLike[] = [];
    for (let i = 0; i < MAX_TOASTS + 3; i++) {
      list = pushToast(list, toast('Info', `notice ${i}`));
    }
    assert.equal(list.length, MAX_TOASTS);
    assert.equal(list[0].title, 'notice 3');
    assert.equal(list[MAX_TOASTS - 1].title, `notice ${MAX_TOASTS + 2}`);
  });

  it('collapses repeats of the same notice onto one counted card', () => {
    // A 16-port bank flapping: one card reading "Port lost (16)" rather than 16
    // cards stacked past the top of the viewport.
    let list: ToastLike[] = [];
    for (let i = 0; i < 16; i++) {
      list = pushToast(list, toast('Warning', 'Port lost', `ttyUSB${i} stopped answering`));
    }
    assert.equal(list.length, 1);
    assert.equal(list[0].count, 16);
    // Newest body wins: a body assembled from 16 port names is unreadable in a
    // 4-second card, and the count is what carries the scale.
    assert.equal(list[0].body, 'ttyUSB15 stopped answering');
  });

  it('never merges two OTP notices', () => {
    // Each code is a distinct thing the operator came to read.
    let list: ToastLike[] = [];
    list = pushToast(list, toast('Otp', 'OTP received', 'from MYTEL', '482931'));
    list = pushToast(list, toast('Otp', 'OTP received', 'from MYTEL', '719815'));
    assert.equal(list.length, 2);
    assert.deepEqual(
      list.map((t) => t.otp),
      ['482931', '719815']
    );
  });

  it('keeps notices apart when only the title differs', () => {
    let list: ToastLike[] = [];
    list = pushToast(list, toast('Warning', 'Delete incomplete'));
    list = pushToast(list, toast('Warning', 'Delete complete'));
    assert.equal(list.length, 2);
  });

  it('keeps notices apart when only the kind differs', () => {
    let list: ToastLike[] = [];
    list = pushToast(list, toast('Success', 'Detect complete'));
    list = pushToast(list, toast('Warning', 'Detect complete'));
    assert.equal(list.length, 2);
  });

  it('moves a coalesced card to the end so a repeat re-announces itself', () => {
    let list: ToastLike[] = [];
    list = pushToast(list, toast('Warning', 'Port lost'));
    list = pushToast(list, toast('Info', 'Scan complete'));
    list = pushToast(list, toast('Warning', 'Port lost'));
    assert.deepEqual(
      list.map((t) => t.title),
      ['Scan complete', 'Port lost']
    );
    assert.equal(list[1].count, 2);
  });

  it('does not mutate the list it was given', () => {
    const original: ToastLike[] = [toast('Info', 'first')];
    const snapshot = [...original];
    pushToast(original, toast('Info', 'second'));
    assert.deepEqual(original, snapshot);
  });

  it('coalescing under the cap does not lose an unrelated card', () => {
    let list: ToastLike[] = [];
    for (let i = 0; i < MAX_TOASTS; i++) {
      list = pushToast(list, toast('Info', `notice ${i}`));
    }
    const before = list.map((t) => t.title);
    list = pushToast(list, toast('Info', 'notice 0'));
    assert.equal(list.length, MAX_TOASTS);
    // "notice 0" moved to the end with a count; everything else is intact.
    assert.deepEqual(list.map((t) => t.title).sort(), before.sort());
    assert.equal(list[MAX_TOASTS - 1].title, 'notice 0');
    assert.equal(list[MAX_TOASTS - 1].count, 2);
  });
});

describe('dismissToast', () => {
  it('removes only the named toast', () => {
    const a = toast('Info', 'a');
    const b = toast('Info', 'b');
    assert.deepEqual(
      dismissToast([a, b], a.id).map((t) => t.title),
      ['b']
    );
  });

  it('is a no-op for an id that is already gone', () => {
    const a = toast('Info', 'a');
    assert.deepEqual(dismissToast([a], 9999), [a]);
  });
});

describe('countSuffix', () => {
  it('is empty for a single notice so the common case reads unchanged', () => {
    assert.equal(countSuffix(toast('Info', 'a')), '');
    assert.equal(countSuffix({ ...toast('Info', 'a'), count: 1 }), '');
  });

  it('shows the count once there is more than one', () => {
    assert.equal(countSuffix({ ...toast('Info', 'a'), count: 16 }), ' (16)');
  });
});
