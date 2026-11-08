// Trimmed from rjsf-team/react-jsonschema-form (Apache-2.0).
// Pinned commit: ad0ea07428f8fde1ed1a958a2383861e242f14d7.
// packages/daisyui/test/DaisyUIForm.test.tsx:108-113 (round 3 false row)
// packages/core/test/LayoutGridField.test.tsx:1396-1413 (round 2 wording row)
import { render, screen } from '@testing-library/react';

describe('ui:options.daisy', () => {
  test('leaves the field wrapper unaffected when not set', () => {
    const { container } = render(<Form schema={schema} validator={validator} />);
    const field = container.querySelector('#root_name')?.closest('.field-template');

    expect(field).not.toHaveAttribute('data-theme');
  });
});

describe('LayoutGridField', () => {
  test('renders nothing when there is no uiSchema', () => {
    const props = getProps();
    render(<LayoutGridField {...props} />);

    // Check for all the possible things rendered by the grid
    const uiComponent = screen.queryByTestId(LayoutGridField.TEST_IDS.uiComponent);
    const field = screen.queryByTestId(LayoutGridField.TEST_IDS.field);
    const layoutMultiSchemaField = screen.queryByTestId(LayoutGridField.TEST_IDS.layoutMultiSchemaField);
    const row = screen.queryByTestId(LayoutGridField.TEST_IDS.uiComponent);
    const col = screen.queryByTestId(LayoutGridField.TEST_IDS.uiComponent);

    // and find none of them
    expect(uiComponent).not.toBeInTheDocument();
    expect(field).not.toBeInTheDocument();
    expect(layoutMultiSchemaField).not.toBeInTheDocument();
    expect(row).not.toBeInTheDocument();
    expect(col).not.toBeInTheDocument();
  });
});
