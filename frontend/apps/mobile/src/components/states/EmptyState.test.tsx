/**
 * EmptyState — unit tests
 * (code-review-mobile-rn-states-untested-hardcoded).
 *
 * `EmptyState` is the shared empty-list placeholder. Unlike `ErrorState` it
 * carries no intrinsic copy — the title/description are required, already
 * localized props supplied by the calling screen (see NotificationsScreen).
 * These tests lock that contract: it renders the caller's title/description,
 * exposes the title as its accessibility label, treats the description and icon
 * as optional, and renders arbitrary children (e.g. a CTA button).
 */

import { render, screen } from '@testing-library/react-native';
import { Text } from 'react-native';
import { EmptyState } from './EmptyState';

// Test fixtures stand in for the already-localized strings a screen would pass.
// Routed through `{expr}` props rather than inline literals so the repo's
// no-hardcoded-strings biome plugin (which rightly flags literal JSX copy in
// production code) stays quiet on intentional test data.
const TITLE = 'No faults found';
const DESCRIPTION = 'All issues have been addressed';
const CAUGHT_UP = "You're all caught up";

describe('EmptyState', () => {
  it('renders the provided title and description', async () => {
    await render(<EmptyState title={TITLE} description={DESCRIPTION} />);
    expect(screen.getByText(TITLE)).toBeTruthy();
    expect(screen.getByText(DESCRIPTION)).toBeTruthy();
  });

  it('exposes the title as the accessibility label', async () => {
    await render(<EmptyState title={CAUGHT_UP} />);
    expect(screen.getByLabelText(CAUGHT_UP)).toBeTruthy();
  });

  it('renders without a description when none is provided', async () => {
    await render(<EmptyState title={TITLE} />);
    expect(screen.getByText(TITLE)).toBeTruthy();
    // No description element — only the title (and default icon) render.
    expect(screen.queryByText(DESCRIPTION)).toBeNull();
  });

  it('renders the provided icon and children', async () => {
    const icon = '🔕';
    const cta = 'Create one';
    await render(
      <EmptyState icon={icon} title={CAUGHT_UP}>
        <Text>{cta}</Text>
      </EmptyState>
    );
    expect(screen.getByText(icon)).toBeTruthy();
    expect(screen.getByText(cta)).toBeTruthy();
  });
});
