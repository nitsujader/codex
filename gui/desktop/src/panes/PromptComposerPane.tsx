interface PromptComposerPaneProps {
  value: string;
  onValueChange: (next: string) => void;
  onSubmit: () => void;
}

export default function PromptComposerPane({ value, onValueChange, onSubmit }: PromptComposerPaneProps) {
  return (
    <section className="pane composer-pane">
      <h2>Prompt Composer</h2>
      <label htmlFor="prompt-input" className="field-label">
        Compose next instruction
      </label>
      <textarea
        id="prompt-input"
        value={value}
        onChange={(event) => onValueChange(event.target.value)}
        rows={7}
        placeholder="Describe the task for the active thread..."
      />
      <div className="composer-actions">
        <button type="button" onClick={onSubmit}>
          Queue Prompt
        </button>
      </div>
    </section>
  );
}
