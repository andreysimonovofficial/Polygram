# Creating Polygram Fast extensions

Polygram Fast discovers extensions automatically while it is running. An extension is a folder containing a `manifest.json`; the folder name must exactly match the manifest `id`. The app checks configured extension folders once per second, so creating or editing a manifest does not require restarting the app.

## Where to put an extension

For development, create the extension under the repository's `extensions` folder:

```text
Polygram Fast/
  extensions/
    my-first-extension/
      manifest.json
```

When running `cargo run` from the repository root, the app watches `.\extensions\`. It also watches an `extensions` folder beside the application executable and the per-user data folder:

```text
%LOCALAPPDATA%\Polygram\Polygram\data\extensions\
```

Each location contains one subfolder per extension. Copy a complete extension folder into one of these locations; it appears in the **Расширения** menu within about one second. A malformed extension is reported in that menu and does not stop other extensions from loading.

## Extension format

Create `manifest.json` as UTF-8 JSON. This first version is declarative: extensions define reusable block templates and their input/output metrics. They do not execute scripts or load native code.

```json
{
  "schemaVersion": 1,
  "id": "sales-dashboard",
  "name": "Sales dashboard",
  "version": "1.0.0",
  "description": "Reusable sales and revenue blocks",
  "blocks": [
    {
      "name": "Lead generation",
      "kind": "Marketing",
      "inputs": [
        {
          "name": "Contact list",
          "value": 1000,
          "required": 500,
          "unit": "contacts"
        }
      ],
      "outputs": [
        {
          "name": "Qualified leads",
          "value": 120,
          "required": 100,
          "max": 250,
          "unit": "leads"
        }
      ]
    }
  ]
}
```

Required top-level fields are `schemaVersion` (`1`), `id`, `name`, `version`, `description`, and a `blocks` array. The ID uses lowercase Latin letters, digits, and single hyphens; it must match the extension folder name. A block requires a non-empty `name`; `kind`, `inputs`, and `outputs` are optional. A metric requires a non-empty `name`. `value` and `required` default to `0`, `max` is optional, and `unit` defaults to `ед.`. Metric values and limits must be finite and non-negative.

Limits: manifest file up to 64 KB, at most 100 blocks per extension, and at most 100 inputs or outputs per block. Keep each extension in a separate folder. Duplicate IDs are rejected; the first valid extension discovered wins.

## Try it

1. Make a folder such as `extensions\sales-dashboard\`.
2. Save the JSON above as `extensions\sales-dashboard\manifest.json`.
3. Wait for the extension count in the **Расширения** menu to update.
4. Choose **Расширения → Sales dashboard · Lead generation** to add the template to the current graph or department.
5. Edit the JSON and save it; the menu refreshes automatically. Remove an extension folder to remove its templates from the menu. Blocks already added to a `.pg` project remain in that project.

The template's blocks and metrics become ordinary project data when added. They can then be edited, connected, moved into a department, linked through a portal, saved, and reopened without the extension folder.
