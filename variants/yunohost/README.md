# TEZAUR for YunoHost (packaging v2)

Static web app, no service. Everything the researcher adds stays in the visitor's own browser.

    sudo yunohost app install /path/to/this/folder
    # or from a Git repository:  yunohost app install https://github.com/<you>/tezaur_ynh

Not yet linted with `package_linter` or installed on a real server — run both before publishing.
The model server (Ollama / Jan) must be reachable from the visitor's browser with CORS enabled.
