#!/usr/bin/env python3
"""Parse Rust Profile structs and generate docs/profiles/all-profiles.md.

Usage:
    python3 scripts/profile_docs.py           # print to stdout
    python3 scripts/profile_docs.py --write   # write to docs/profiles/all-profiles.md
"""

from __future__ import annotations

import argparse
import pathlib
import re
import sys
from dataclasses import dataclass, field
from typing import Optional

ROOT = pathlib.Path(__file__).resolve().parents[1]
CORE_SRC = ROOT / "crates" / "worldwake-core" / "src"
SPAWN_FILE = ROOT / "crates" / "worldwake-cli" / "src" / "scenario" / "mod.rs"
OUTPUT_DIR = ROOT / "docs" / "profiles"
OUTPUT_PATH = OUTPUT_DIR / "all-profiles.md"

# Exclude item-related profiles (not agent behavior profiles).
EXCLUDE_PROFILES = {
    "CommodityPhysicalProfile",
    "CommodityConsumableProfile",
    "CommodityTreatmentProfile",
    "CombatWeaponProfile",
    "UniqueItemPhysicalProfile",
}

STRUCT_RE = re.compile(r"^pub struct (\w+Profile)\b")
FIELD_RE = re.compile(r"^\s+pub (\w+):\s*(.+?),?\s*$")
DOC_COMMENT_RE = re.compile(r"^\s*///\s?(.*)")
ATTR_RE = re.compile(r"^\s*#\[")
DEFAULT_IMPL_RE = re.compile(r"^impl Default for (\w+Profile)\b")
DEFAULT_FIELD_RE = re.compile(
    r"^\s+(\w+):\s*(.+?),?\s*$"
)


@dataclass
class FieldInfo:
    name: str
    field_type: str
    doc: str
    default_value: Optional[str] = None


@dataclass
class ProfileInfo:
    name: str
    doc: str
    source_file: str
    line_number: int
    fields: list[FieldInfo] = field(default_factory=list)
    has_default: bool = False
    category: str = "unknown"  # "universal" or "optional"


def parse_profiles(src_dir: pathlib.Path) -> list[ProfileInfo]:
    """Parse all *Profile structs from Rust source files."""
    profiles: list[ProfileInfo] = []

    for rs_file in sorted(src_dir.glob("*.rs")):
        lines = rs_file.read_text().splitlines()
        i = 0
        while i < len(lines):
            line = lines[i]

            # Detect struct definition.
            m = STRUCT_RE.match(line)
            if m and m.group(1) not in EXCLUDE_PROFILES:
                name = m.group(1)
                struct_line = i + 1  # 1-indexed

                # Walk backwards to collect doc comments (skip attributes).
                doc_lines: list[str] = []
                j = i - 1
                while j >= 0:
                    dm = DOC_COMMENT_RE.match(lines[j])
                    am = ATTR_RE.match(lines[j])
                    if dm:
                        doc_lines.insert(0, dm.group(1))
                        j -= 1
                    elif am:
                        j -= 1  # skip attributes
                    else:
                        break

                struct_doc = " ".join(doc_lines).strip()

                # Parse fields until closing brace.
                profile_fields: list[FieldInfo] = []
                i += 1
                while i < len(lines) and not lines[i].startswith("}"):
                    field_line = lines[i]

                    # Collect doc comments above the field.
                    fm = FIELD_RE.match(field_line)
                    if fm:
                        field_doc_lines: list[str] = []
                        k = i - 1
                        # Walk back over doc comments and attributes.
                        while k >= 0:
                            fdm = DOC_COMMENT_RE.match(lines[k])
                            fam = ATTR_RE.match(lines[k])
                            if fdm:
                                field_doc_lines.insert(0, fdm.group(1))
                                k -= 1
                            elif fam:
                                k -= 1
                            else:
                                break

                        field_doc = " ".join(field_doc_lines).strip()
                        field_name = fm.group(1)
                        field_type = fm.group(2).rstrip(",").strip()

                        profile_fields.append(
                            FieldInfo(
                                name=field_name,
                                field_type=field_type,
                                doc=field_doc,
                            )
                        )
                    i += 1

                rel_path = rs_file.relative_to(ROOT)
                profiles.append(
                    ProfileInfo(
                        name=name,
                        doc=struct_doc,
                        source_file=str(rel_path),
                        line_number=struct_line,
                        fields=profile_fields,
                    )
                )
            else:
                i += 1

    return profiles


def parse_defaults(src_dir: pathlib.Path, profiles: list[ProfileInfo]) -> None:
    """Find Default impls and extract default values for profile fields."""
    profile_map = {p.name: p for p in profiles}

    for rs_file in sorted(src_dir.glob("*.rs")):
        lines = rs_file.read_text().splitlines()
        i = 0
        while i < len(lines):
            m = DEFAULT_IMPL_RE.match(lines[i])
            if m and m.group(1) in profile_map:
                profile = profile_map[m.group(1)]
                profile.has_default = True

                # Find the Self { ... } block inside the default fn.
                # Look for "Self {" or "Self::new(" pattern.
                j = i + 1
                in_self_block = False
                brace_depth = 0
                while j < len(lines):
                    sline = lines[j].strip()

                    if not in_self_block:
                        if "Self {" in sline or "Self{" in sline:
                            in_self_block = True
                            brace_depth = 1
                        elif sline == "}":
                            # End of impl block without finding Self {}
                            break
                    else:
                        brace_depth += sline.count("{") - sline.count("}")
                        if brace_depth <= 0:
                            break

                        # Try to parse field: value lines.
                        fm = DEFAULT_FIELD_RE.match(lines[j])
                        if fm:
                            field_name = fm.group(1)
                            field_value = fm.group(2).rstrip(",").strip()
                            for f in profile.fields:
                                if f.name == field_name:
                                    f.default_value = field_value
                                    break
                    j += 1
            i += 1


def classify_profiles(spawn_file: pathlib.Path, profiles: list[ProfileInfo]) -> None:
    """Classify profiles as universal or optional by parsing spawn_agent()."""
    if not spawn_file.exists():
        return

    text = spawn_file.read_text()

    # Extract just the spawn_agent function body.
    start = text.find("fn spawn_agent(")
    if start < 0:
        return

    # Find matching closing brace.
    brace_depth = 0
    end = start
    found_opening = False
    for idx in range(start, len(text)):
        if text[idx] == "{":
            brace_depth += 1
            found_opening = True
        elif text[idx] == "}":
            brace_depth -= 1
            if found_opening and brace_depth == 0:
                end = idx
                break
    body = text[start:end]

    for profile in profiles:
        # Convert CamelCase profile name to snake_case field name patterns.
        # AgentDef fields sometimes drop the "_profile" suffix, so try both.
        snake = _camel_to_snake(profile.name)
        candidates = [snake]
        if snake.endswith("_profile"):
            candidates.append(snake[: -len("_profile")])

        matched = False
        for field_name in candidates:
            # Use \s* to handle multi-line method chains like:
            #   agent_def
            #       .exploration_profile
            #       .map_or_else(
            universal_re = re.compile(
                rf"agent_def\s*\.\s*{field_name}\b[^;]*?"
                rf"\.(?:unwrap_or(?:_default)?|map_or_else)\s*\(",
                re.DOTALL,
            )
            optional_re = re.compile(
                rf"if let Some\([^)]*\)\s*=\s*agent_def\s*\.\s*{field_name}\b",
                re.DOTALL,
            )

            if universal_re.search(body):
                profile.category = "universal"
                matched = True
                break
            elif optional_re.search(body):
                profile.category = "optional"
                matched = True
                break

        if not matched:
            # Not referenced in spawn_agent — might be non-agent or
            # attached elsewhere.
            profile.category = "other"


def _camel_to_snake(name: str) -> str:
    """Convert CamelCase to snake_case."""
    s1 = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", name)
    return re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", s1).lower()


def render_markdown(profiles: list[ProfileInfo]) -> str:
    """Render the complete markdown document."""
    universal = sorted(
        [p for p in profiles if p.category == "universal"], key=lambda p: p.name
    )
    optional = sorted(
        [p for p in profiles if p.category == "optional"], key=lambda p: p.name
    )
    other = sorted(
        [p for p in profiles if p.category == "other"], key=lambda p: p.name
    )

    lines: list[str] = []
    lines.append(
        "<!-- This file is generated by `python3 scripts/profile_docs.py --write`. -->"
    )
    lines.append("<!-- Do not hand-edit it. -->")
    lines.append("")
    lines.append("# Agent Profile Reference")
    lines.append("")

    # Table of contents.
    lines.append("## Table of Contents")
    lines.append("")

    if universal:
        lines.append("### Universal Profiles")
        lines.append("")
        lines.append("Always applied to every agent with defaults. "
                      "Scenario definitions may override individual fields.")
        lines.append("")
        for p in universal:
            anchor = p.name.lower()
            summary = _first_sentence(p.doc) or "(no description)"
            lines.append(f"- [{p.name}](#{anchor}) — {summary}")
        lines.append("")

    if optional:
        lines.append("### Optional Profiles")
        lines.append("")
        lines.append("Applied only when the scenario definition includes them. "
                      "Agents without these profiles lack the associated capability.")
        lines.append("")
        for p in optional:
            anchor = p.name.lower()
            summary = _first_sentence(p.doc) or "(no description)"
            lines.append(f"- [{p.name}](#{anchor}) — {summary}")
        lines.append("")

    if other:
        lines.append("### Non-Agent Profiles")
        lines.append("")
        lines.append("Attached to non-agent entities (places, offices, etc.) "
                      "or not referenced in the agent spawn path.")
        lines.append("")
        for p in other:
            anchor = p.name.lower()
            summary = _first_sentence(p.doc) or "(no description)"
            lines.append(f"- [{p.name}](#{anchor}) — {summary}")
        lines.append("")

    lines.append("---")
    lines.append("")

    # Profile sections.
    all_profiles = universal + optional + other
    for p in all_profiles:
        lines.extend(_render_profile(p))

    return "\n".join(lines)


def _render_profile(p: ProfileInfo) -> list[str]:
    """Render a single profile section."""
    lines: list[str] = []
    lines.append(f"## {p.name}")
    lines.append("")

    category_label = {
        "universal": "Universal (always applied with defaults)",
        "optional": "Optional (scenario-specified only)",
        "other": "Non-agent / special-purpose",
    }.get(p.category, p.category)
    lines.append(f"**Category**: {category_label}")
    lines.append("")
    lines.append(f"**Source**: `{p.source_file}:{p.line_number}`")
    lines.append("")

    if p.doc:
        lines.append(p.doc)
        lines.append("")

    if p.fields:
        lines.append("| Field | Type | Description |")
        lines.append("|-------|------|-------------|")
        for f in p.fields:
            desc = f.doc or "*(undocumented)*"
            ftype = _escape_pipes(f.field_type)
            default_suffix = ""
            if f.default_value is not None:
                default_suffix = f" (default: `{f.default_value}`)"
            lines.append(
                f"| `{f.name}` | `{ftype}` | {desc}{default_suffix} |"
            )
        lines.append("")
    else:
        lines.append("*(no public fields)*")
        lines.append("")

    lines.append("---")
    lines.append("")
    return lines


def _first_sentence(text: str) -> str:
    """Extract the first sentence from a doc comment."""
    if not text:
        return ""
    # Take up to the first period followed by space or end.
    m = re.match(r"^(.+?\.)\s", text)
    if m:
        return m.group(1)
    return text


def _escape_pipes(text: str) -> str:
    """Escape pipe characters for markdown tables."""
    return text.replace("|", "\\|")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate profile documentation from Rust source."
    )
    parser.add_argument(
        "--write",
        action="store_true",
        help="Write output to docs/profiles/all-profiles.md",
    )
    args = parser.parse_args()

    profiles = parse_profiles(CORE_SRC)
    parse_defaults(CORE_SRC, profiles)
    classify_profiles(SPAWN_FILE, profiles)

    md = render_markdown(profiles)

    if args.write:
        OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
        OUTPUT_PATH.write_text(md + "\n")
        print(f"Wrote {OUTPUT_PATH}")
        # Report gaps.
        _report_gaps(profiles)
    else:
        print(md)


def _report_gaps(profiles: list[ProfileInfo]) -> None:
    """Print warnings for profiles or fields missing doc comments."""
    gaps = 0
    for p in profiles:
        if not p.doc:
            print(f"  WARNING: {p.name} has no struct-level doc comment", file=sys.stderr)
            gaps += 1
        for f in p.fields:
            if not f.doc:
                print(
                    f"  WARNING: {p.name}.{f.name} has no doc comment",
                    file=sys.stderr,
                )
                gaps += 1
    if gaps:
        print(f"\n  {gaps} documentation gap(s) found.", file=sys.stderr)
    else:
        print("  No documentation gaps found.", file=sys.stderr)


if __name__ == "__main__":
    main()
