// RAW is an aggregate video filter. Concrete catalog codec names still feed
// folder rules, so selecting this does not merge the codec directories.
export function videoCodecChoices(codecs:readonly string[]=[],selected=''):string[] {
  return Array.from(new Set(['RAW',...codecs,...(selected?[selected]:[])]));
}
