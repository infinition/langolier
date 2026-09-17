/// The creature, mounted on an element the caller owns.
export declare function mountLangolier(
  el: HTMLElement | null,
  options?: Record<string, unknown>,
): {
  /// Swallows a burst of documents, once.
  feed(amount?: number): void;
  /// While true, documents keep arriving: something is being ingested.
  setFeeding(state: boolean): void;
  destroy(): void;
} | null;
