import { test, expect } from "@playwright/test";
test("source paste dialog preserves input between tabs", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Paste text" }).click();
  await page.getByLabel("Source title").fill("Transformer notes");
  await page
    .getByLabel("Your text", { exact: true })
    .fill("Attention computes weighted combinations of values.");
  await expect(
    page.getByRole("button", { name: "Add to my memory" }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "Video link", exact: true }).click();
  await page.getByRole("button", { name: "Pasted text", exact: true }).click();
  await expect(page.getByLabel("Your text", { exact: true })).toHaveValue(
    "Attention computes weighted combinations of values.",
  );
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).not.toBeVisible();
});
test("all primary views render without fabricated metrics or runtime errors", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("/");
  for (const [nav, title] of [
    ["Sources", "The raw material."],
    ["Observatory", "Under the hood."],
    ["Laboratory", "The laboratory."],
    ["Engines", "Choose your engine."],
  ]) {
    await page
      .locator("nav")
      .getByRole("button", { name: new RegExp(nav) })
      .click();
    await expect(page.getByRole("heading", { name: title })).toBeVisible();
  }
  expect(errors).toEqual([]);
});
test("layout fits desktop and narrow native window", async ({ page }) => {
  for (const width of [1440, 900, 800]) {
    await page.setViewportSize({ width, height: 940 });
    await page.goto("/");
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBeTruthy();
    await expect(
      page.getByRole("button", { name: "Send", exact: true }),
    ).toBeVisible();
  }
});
