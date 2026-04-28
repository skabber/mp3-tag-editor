# MP3 Tag Editor - Future Improvements

## High Priority

- [ ] **Dark Mode Toggle**
  - Add a theme switch to toggle between light and dark modes
  - Use CSS variables for theme colors
  - Store preference in localStorage

- [ ] **Drag-and-Drop Chapter Reordering**
  - Allow users to reorder chapters by dragging
  - Update chapter timestamps accordingly
  - Visual feedback during drag operations

## Medium Priority

- [ ] **Reset to Original Tags**
  - Store original tag values when loading a file
  - Add "Reset" button to restore original values
  - Confirmation dialog for reset action

- [ ] **Keyboard Shortcuts**
  - Ctrl+S / Cmd+S to save tags
  - Ctrl+R / Cmd+R to reset form
  - Esc to close error messages

- [ ] **Bulk Chapter Operations**
  - Select multiple chapters for deletion
  - Copy/paste chapter settings between chapters

## Low Priority

- [ ] **Waveform Visualization**
  - Display audio waveform for precise chapter timing
  - Click on waveform to set chapter start/end times

- [ ] **Import/Export Chapter Lists**
  - Export chapters as JSON for backup
  - Import chapter lists from JSON files

- [ ] **Tag Presets**
  - Save frequently used tag combinations
  - Apply presets with one click

## Technical Debt

- [ ] **Unit Tests**
  - Add tests for `parse_mp3_data` function
  - Test chapter validation logic
  - Test edge cases in time parsing

- [ ] **Code Splitting**
  - Extract tag parsing logic into separate module
  - Create a `tags.rs` for tag manipulation functions
  - Move UI components into separate functions

- [ ] **Performance Optimization**
  - Lazy loading for large MP3 files
  - Debounce URL input validation
  - Memoize expensive calculations

## Nice to Have

- [ ] **Multi-language Support**
  - i18n for UI strings
  - Support common languages

- [ ] **Export to Different Formats**
  - Export tags as JSON
  - Copy tags to clipboard

- [ ] **Undo/Redo**
  - History stack for tag changes
  - Cmd+Z / Cmd+Shift+Z support