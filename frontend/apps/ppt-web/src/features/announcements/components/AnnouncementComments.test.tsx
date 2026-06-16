/**
 * AnnouncementComments tests (feat-6-3 — Story 6.3 comments & discussion web UI).
 *
 * Mirrors the #1190/#1196 coverage pattern: the component shipped without a
 * dedicated test. These cover the load-bearing behaviour:
 *  - top-level comments + nested replies render (threading)
 *  - the reply affordance is capped at depth 3 (matches the marginLeft cap and
 *    the backend's max threading depth — see screen-map note)
 *  - deleted comments collapse to a tombstone (no content / actions)
 *  - composing a new top-level comment wires to onAddComment with no parentId
 *  - replying wires to onAddComment with the parent comment id
 *  - delete affordance: own comment OR manager; hidden otherwise
 *  - disabled hides all compose/reply/delete affordances
 *  - empty + loading states
 */
/// <reference types="vitest/globals" />

import type { CommentWithAuthor } from '@ppt/api-client';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { AnnouncementComments } from './AnnouncementComments';

function comment(overrides: Partial<CommentWithAuthor> = {}): CommentWithAuthor {
  return {
    id: 'c-1',
    announcementId: 'ann-1',
    userId: 'user-1',
    content: 'First!',
    authorName: 'Alice Tenant',
    isDeleted: false,
    createdAt: '2026-06-01T00:00:00Z',
    updatedAt: '2026-06-01T00:00:00Z',
    ...overrides,
  };
}

describe('AnnouncementComments', () => {
  it('renders top-level comments with their authors and content', () => {
    render(
      <AnnouncementComments
        comments={[comment({ id: 'c-1', content: 'Hello world', authorName: 'Alice Tenant' })]}
        total={1}
        onAddComment={vi.fn()}
        onDeleteComment={vi.fn()}
      />
    );

    expect(screen.getByText('Hello world')).toBeInTheDocument();
    expect(screen.getByText('Alice Tenant')).toBeInTheDocument();
    // total count badge
    expect(screen.getByRole('heading', { name: /discussion/i })).toBeInTheDocument();
    expect(screen.getByText('1')).toBeInTheDocument();
  });

  it('renders nested replies (threading)', () => {
    render(
      <AnnouncementComments
        comments={[
          comment({
            id: 'c-1',
            content: 'Parent comment',
            replies: [
              comment({ id: 'c-2', parentId: 'c-1', content: 'A reply', authorName: 'Bob' }),
            ],
          }),
        ]}
        total={2}
        onAddComment={vi.fn()}
        onDeleteComment={vi.fn()}
      />
    );

    expect(screen.getByText('Parent comment')).toBeInTheDocument();
    expect(screen.getByText('A reply')).toBeInTheDocument();
    expect(screen.getByText('Bob')).toBeInTheDocument();
  });

  it('caps the reply affordance at depth 3 (max threading depth)', () => {
    // Build a 5-deep chain: depths 0,1,2,3,4. Reply button should appear on
    // depths 0..2 and disappear at depth 3 and below.
    const deepChain = comment({
      id: 'd0',
      content: 'depth-0',
      replies: [
        comment({
          id: 'd1',
          content: 'depth-1',
          replies: [
            comment({
              id: 'd2',
              content: 'depth-2',
              replies: [
                comment({
                  id: 'd3',
                  content: 'depth-3',
                  replies: [comment({ id: 'd4', content: 'depth-4' })],
                }),
              ],
            }),
          ],
        }),
      ],
    });

    render(
      <AnnouncementComments
        comments={[deepChain]}
        total={5}
        onAddComment={vi.fn()}
        onDeleteComment={vi.fn()}
      />
    );

    // depths 0,1,2 each render a "Reply" button -> 3 buttons total.
    expect(screen.getAllByRole('button', { name: 'Reply' })).toHaveLength(3);
    // The deepest comments still render their content.
    expect(screen.getByText('depth-3')).toBeInTheDocument();
    expect(screen.getByText('depth-4')).toBeInTheDocument();
  });

  it('renders a tombstone for deleted comments without content or actions', () => {
    render(
      <AnnouncementComments
        comments={[comment({ id: 'c-1', content: 'secret', isDeleted: true })]}
        total={1}
        currentUserId="user-1"
        onAddComment={vi.fn()}
        onDeleteComment={vi.fn()}
      />
    );

    expect(screen.getByText('[This comment has been deleted]')).toBeInTheDocument();
    expect(screen.queryByText('secret')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Reply' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Delete' })).not.toBeInTheDocument();
  });

  it('submits a new top-level comment with no parentId', async () => {
    const user = userEvent.setup();
    const onAddComment = vi.fn().mockResolvedValue(undefined);

    render(
      <AnnouncementComments
        comments={[]}
        total={0}
        onAddComment={onAddComment}
        onDeleteComment={vi.fn()}
      />
    );

    await user.type(screen.getByLabelText('New comment'), 'A fresh take');
    await user.click(screen.getByRole('button', { name: 'Post comment' }));

    expect(onAddComment).toHaveBeenCalledWith('A fresh take');
  });

  it('replies wire onAddComment with the parent comment id', async () => {
    const user = userEvent.setup();
    const onAddComment = vi.fn().mockResolvedValue(undefined);

    render(
      <AnnouncementComments
        comments={[comment({ id: 'parent-1', content: 'reply to me' })]}
        total={1}
        onAddComment={onAddComment}
        onDeleteComment={vi.fn()}
      />
    );

    await user.click(screen.getByRole('button', { name: 'Reply' }));
    await user.type(screen.getByLabelText('Reply content'), 'my reply');
    await user.click(screen.getByRole('button', { name: 'Post reply' }));

    expect(onAddComment).toHaveBeenCalledWith('my reply', 'parent-1');
  });

  it('shows the delete affordance for the comment owner', () => {
    render(
      <AnnouncementComments
        comments={[comment({ id: 'c-1', userId: 'user-1' })]}
        total={1}
        currentUserId="user-1"
        onAddComment={vi.fn()}
        onDeleteComment={vi.fn()}
      />
    );

    expect(screen.getByRole('button', { name: 'Delete' })).toBeInTheDocument();
  });

  it('hides the delete affordance for non-owners who are not managers', () => {
    render(
      <AnnouncementComments
        comments={[comment({ id: 'c-1', userId: 'someone-else' })]}
        total={1}
        currentUserId="user-1"
        onAddComment={vi.fn()}
        onDeleteComment={vi.fn()}
      />
    );

    expect(screen.queryByRole('button', { name: 'Delete' })).not.toBeInTheDocument();
  });

  it('shows the delete affordance to a manager for any comment', async () => {
    const user = userEvent.setup();
    const onDeleteComment = vi.fn().mockResolvedValue(undefined);

    render(
      <AnnouncementComments
        comments={[comment({ id: 'c-1', userId: 'someone-else' })]}
        total={1}
        currentUserId="manager-1"
        isManager
        onAddComment={vi.fn()}
        onDeleteComment={onDeleteComment}
      />
    );

    await user.click(screen.getByRole('button', { name: 'Delete' }));
    expect(onDeleteComment).toHaveBeenCalledWith('c-1');
  });

  it('hides all compose/reply/delete affordances when disabled', () => {
    render(
      <AnnouncementComments
        comments={[comment({ id: 'c-1', userId: 'user-1' })]}
        total={1}
        currentUserId="user-1"
        isManager
        disabled
        onAddComment={vi.fn()}
        onDeleteComment={vi.fn()}
      />
    );

    expect(screen.queryByLabelText('New comment')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Reply' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Delete' })).not.toBeInTheDocument();
    // existing content still shown
    expect(screen.getByText('First!')).toBeInTheDocument();
  });

  it('renders the empty state when there are no comments', () => {
    render(
      <AnnouncementComments
        comments={[]}
        total={0}
        onAddComment={vi.fn()}
        onDeleteComment={vi.fn()}
      />
    );

    expect(
      screen.getByText('No comments yet. Be the first to start the discussion!')
    ).toBeInTheDocument();
  });

  it('renders the loading state instead of empty/list while loading', () => {
    render(
      <AnnouncementComments
        comments={[]}
        total={0}
        isLoading
        onAddComment={vi.fn()}
        onDeleteComment={vi.fn()}
      />
    );

    expect(screen.getByLabelText('Loading comments')).toBeInTheDocument();
    expect(
      screen.queryByText('No comments yet. Be the first to start the discussion!')
    ).not.toBeInTheDocument();
  });

  it('submits a comment via Ctrl+Enter', async () => {
    const user = userEvent.setup();
    const onAddComment = vi.fn().mockResolvedValue(undefined);

    render(
      <AnnouncementComments
        comments={[]}
        total={0}
        onAddComment={onAddComment}
        onDeleteComment={vi.fn()}
      />
    );

    const textarea = screen.getByLabelText('New comment');
    await user.type(textarea, 'keyboard submit');
    await user.keyboard('{Control>}{Enter}{/Control}');

    expect(onAddComment).toHaveBeenCalledWith('keyboard submit');
  });

  it('keeps a reply scoped to its own parent in a multi-comment thread', async () => {
    const user = userEvent.setup();
    const onAddComment = vi.fn().mockResolvedValue(undefined);

    render(
      <AnnouncementComments
        comments={[
          comment({ id: 'p-1', content: 'first parent' }),
          comment({ id: 'p-2', content: 'second parent' }),
        ]}
        total={2}
        onAddComment={onAddComment}
        onDeleteComment={vi.fn()}
      />
    );

    // Open the reply form under the second comment.
    const secondItem = screen.getByText('second parent').closest('.ann-comment') as HTMLElement;
    await user.click(within(secondItem).getByRole('button', { name: 'Reply' }));
    await user.type(screen.getByLabelText('Reply content'), 'reply to second');
    await user.click(screen.getByRole('button', { name: 'Post reply' }));

    expect(onAddComment).toHaveBeenCalledWith('reply to second', 'p-2');
  });
});
