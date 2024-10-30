import { useRef, useState, useEffect } from "react";
import { z } from "zod";
import { zCollectedData } from "@/utils/api";
import CodeMirror, {
  ReactCodeMirrorRef,
  Decoration,
  StateEffect,
  StateField,
  Range,
  EditorView,
  SelectionRange,
} from "@uiw/react-codemirror";
import { vscodeDark, vscodeLight } from "@uiw/codemirror-theme-vscode";
import { rustLanguage } from "@codemirror/lang-rust";

export type Effect = {
  from: number;
  until: number;
  style: string;
};

type Props = {
  code: string;
  setCode: (code: string) => void;
  analyzed?: z.infer<typeof zCollectedData>;
  effects: Effect[];
  onCursorPos?: (pos: SelectionRange) => void;
};

const effectDef = StateEffect.define<Range<Decoration>[] | null>();
const effectExt = StateField.define({
  create: () => Decoration.none,
  update: (value, transaction) => {
    value = value.map(transaction.changes);
    for (const effect of transaction.effects) {
      if (effect.is(effectDef)) {
        if (effect.value === null) {
          return Decoration.none;
        } else {
          value = value.update({ add: effect.value, sort: true });
        }
      }
    }
    return value;
  },
  provide: (sf) => EditorView.decorations.from(sf),
});

export default ({ code, setCode, effects, onCursorPos }: Props) => {
  const cmRef = useRef<ReactCodeMirrorRef>(null);
  const [darkTheme, setDarkTheme] = useState(true);

  const view = cmRef.current?.view;
  const updateEffect = (effects: Range<Decoration>[] | null) => {
    view?.dispatch({ effects: effectDef.of(effects) });
  };

  useEffect(() => {
    updateEffect(null);
    const toApply = effects
      .filter((ef) => ef.from < ef.until)
      .map((effect) =>
        Decoration.mark({
          attributes: { style: effect.style },
        }).range(effect.from, effect.until),
      );
    updateEffect(toApply);
  }, [effects]);

  return (
    <div className="w-full h-full flex flex-col">
      <div>
        <p>
          <button
            type="button"
            disabled={true}
            onClick={() => setDarkTheme((prev) => !prev)}
          >
            toggle theme (disabled)
          </button>
        </p>
      </div>
      <CodeMirror
        className="w-full h-full"
        width="100%"
        height="100%"
        value={code}
        extensions={[rustLanguage, effectExt]}
        ref={cmRef}
        theme={darkTheme ? vscodeDark : vscodeLight}
        onChange={(value, viewUpdate) => {
          setCode(value);
        }}
        onUpdate={(viewUpdate) => {
          onCursorPos?.(viewUpdate.view.state.selection.main);
        }}
      />
    </div>
  );
};
