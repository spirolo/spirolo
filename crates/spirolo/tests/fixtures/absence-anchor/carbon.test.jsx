/**
 * Copyright IBM Corp. 2023, 2026
 *
 * This source code is licensed under the Apache-2.0 license found in the
 * LICENSE file in the root directory of this source tree.
 */
// Trimmed from carbon-design-system/carbon at
// 40f8d5cfaed733feefa2e7c39b8e9eea1795d97d.
// packages/react/src/components/Dialog/Dialog-test.js:47-50 (round 3 false)
// packages/react/src/components/AddSelect/__tests__/AddSelect-test.js:1587-1605
// (round 1 true).
import React from 'react';
import { render, screen } from '@testing-library/react';

describe('Dialog', () => {
  describe('renders as expected - Component API', () => {
    it('supports non-modal', () => {
      const { container } = render(<Dialog />);
      expect(container.firstChild).not.toHaveClass(`${prefix}--dialog--modal`);
    });
  });
});

describe('AddSelect.SelectionSummaryItem', () => {
  it('does not render arbitrary item fields (e.g. value, Region) via defaultContent', () => {
    // defaultContent now renders ONLY itemDetails — arbitrary item props that
    // are developer identifiers must not appear as visible UI labels.
    const richItem = {
      id: 'r1',
      title: 'Rich item',
      value: 'rv',
      Region: 'US East',
    };
    render(
      <AddSelect>
        <AddSelect.SelectionSummaryItem item={richItem} />
      </AddSelect>
    );
    expect(screen.queryByText('Region')).not.toBeInTheDocument();
    expect(screen.queryByText('US East')).not.toBeInTheDocument();
    expect(screen.queryByText('value')).not.toBeInTheDocument();
    expect(screen.queryByText('rv')).not.toBeInTheDocument();
  });
});
