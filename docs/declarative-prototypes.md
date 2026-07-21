# Declarative prototype security

UX agents emit a strict JSON document, not HTML, JavaScript, CSS, a component import, or executable expression. The backend rejects unknown fields, duplicate node IDs, invalid state references, unsupported actions, oversized/deep trees, and non-allowlisted primitives. The frontend validates again before rendering.

Allowed primitives are layout, stack, split pane, tabs, card, text, status badge, progress indicator, button, form field, table, timeline, graph/diff/code placeholder, modal, and callout. Prototype actions may only update document-local scalar state. There is no DOM injection, filesystem access, command execution, network access, package loading, dynamic import, or repository mutation.

Backend validation is the security boundary. Frontend checks are defense in depth and improve error messages; they are not authority.
