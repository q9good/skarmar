"""Inspect a complete Tencent snapshot without printing training or caregiver text."""

import argparse
import json
import os
from collections import Counter
from pathlib import Path


def reference_ids(value):
    if not isinstance(value, dict):
        return []
    result = []
    for item in value.get("items", []):
        if isinstance(item, str) and item:
            result.append(item)
        elif isinstance(item, dict):
            candidate = item.get("record_id") or item.get("id")
            if isinstance(candidate, str) and candidate:
                result.append(candidate)
    return result


def audit(snapshot):
    if not isinstance(snapshot.get("sheets"), dict):
        raise ValueError("Expected a Tencent snapshot containing sheets")
    sheets = snapshot["sheets"]
    records_by_sheet = {}
    summaries, incomplete, unresolved, unsupported = [], [], [], []
    fields_by_sheet = {}
    for sheet_id, sheet in sheets.items():
        summary = {"sheet_id": sheet_id}
        for kind in ("fields", "views", "records"):
            page = sheet.get(kind, {})
            items = page.get(kind, [])
            if not isinstance(items, list):
                raise ValueError(f"Unsupported {kind} page in {sheet_id}")
            summary[kind + "_count"] = len(items)
            # Missing pagination metadata cannot establish a complete export.
            if page.get("has_more") is not False or page.get("total") != len(items) or page.get("error"):
                incomplete.append({"sheet_id": sheet_id, "page": kind})
        records = sheet.get("records", {}).get("records", [])
        record_ids = [r["record_id"] for r in records]
        if len(record_ids) != len(set(record_ids)):
            incomplete.append({"sheet_id": sheet_id, "page": "duplicate_records"})
        records_by_sheet[sheet_id] = set(record_ids)
        fields = sheet.get("fields", {}).get("fields", [])
        titles = [f["field_title"] for f in fields]
        if len(titles) != len(set(titles)):
            incomplete.append({"sheet_id": sheet_id, "page": "ambiguous_field_titles"})
        field_ids = [f["field_id"] for f in fields]
        if len(field_ids) != len(set(field_ids)):
            incomplete.append({"sheet_id": sheet_id, "page": "duplicate_field_ids"})
        fields_by_sheet[sheet_id] = {f["field_id"]: f for f in fields}
        summary["field_types"] = dict(Counter(f["field_type"] for f in fields))
        summaries.append(summary)
    edges = set()
    obsolete_fields = []
    for sheet_id, sheet in sheets.items():
        fields = fields_by_sheet[sheet_id]
        title_counts = Counter(field["field_title"] for field in fields.values())
        by_title = {field["field_title"]: field for field in fields.values() if title_counts[field["field_title"]] == 1}
        for field in fields.values():
            if field["field_type"] not in ("reference", "twoWayLinkRecords"):
                continue
            key = "property_reference" if field["field_type"] == "reference" else "property_two_way_link_records"
            target_sheet = field.get(key, {}).get("sub_id")
            if target_sheet and target_sheet not in sheets:
                obsolete_fields.append({"sheet_id": sheet_id, "field_id": field["field_id"], "target_sheet_id": target_sheet})
        for record in sheet["records"]["records"]:
            for value in record["field_values"]:
                if "reference_value" not in value:
                    continue
                field_meta = value.get("field", {})
                field_key = (field_meta.get("field_id") or field_meta.get("field_title")) if isinstance(field_meta, dict) else field_meta
                # Tencent record responses can identify a field by title rather than by ID.
                field = fields.get(field_key) or by_title.get(field_key)
                if not field:
                    unsupported.append({"sheet_id": sheet_id, "reason": "unmapped_reference_field"})
                    continue
                field_id = field["field_id"]
                if field["field_type"] not in ("reference", "twoWayLinkRecords"):
                    continue
                key = "property_reference" if field["field_type"] == "reference" else "property_two_way_link_records"
                target_sheet = field.get(key, {}).get("sub_id")
                refs = value["reference_value"]
                if not isinstance(refs, dict) or not isinstance(refs.get("items"), list):
                    unsupported.append({"sheet_id": sheet_id, "field_id": field_id})
                    continue
                for item in refs["items"]:
                    candidate = (item.get("record_id") or item.get("id")) if isinstance(item, dict) else item
                    if not isinstance(candidate, str) or not candidate:
                        unsupported.append({"sheet_id": sheet_id, "field_id": field_id})
                for target_id in reference_ids(refs):
                    edge = (sheet_id, record["record_id"], field_id, target_sheet, target_id)
                    edges.add(edge)
                    if target_sheet not in records_by_sheet or target_id not in records_by_sheet[target_sheet]:
                        unresolved.append({"sheet_id": sheet_id, "record_id": record["record_id"], "field_id": field_id,
                            "target_sheet_id": target_sheet, "target_record_id": target_id,
                            "reason": "missing_sheet" if target_sheet not in records_by_sheet else "missing_record"})
    return {"captured_at": snapshot.get("captured_at"), "complete_pages": not incomplete,
        "incomplete_pages": incomplete, "sheets": summaries, "reference_edge_count": len(edges),
        "obsolete_relation_fields": obsolete_fields, "unresolved_references": unresolved,
        "unsupported_reference_values": unsupported}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("snapshot", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    checkout = Path(__file__).resolve().parents[1]
    output = args.output.resolve()
    if output == checkout or checkout in output.parents or any(
        (parent / '.git').is_file() or (parent / '.git' / 'HEAD').is_file() for parent in output.parents
    ):
        parser.error("Audit reports containing source identifiers must stay outside the checkout")
    report = audit(json.loads(args.snapshot.read_text(encoding="utf-8")))
    output.parent.mkdir(parents=True, exist_ok=True)
    temp = output.with_suffix(output.suffix + '.tmp')
    with open(temp, 'w', encoding='utf-8', opener=lambda name, flags: os.open(name, flags, 0o600)) as out:
        json.dump(report, out, ensure_ascii=False, indent=2)
    temp.replace(output)
    output.chmod(0o600)
    print(json.dumps({"complete_pages": report["complete_pages"],
        "table_record_counts": [s["records_count"] for s in report["sheets"]],
        "reference_edge_count": report["reference_edge_count"],
        "obsolete_relation_field_count": len(report["obsolete_relation_fields"]),
        "unresolved_reference_count": len(report["unresolved_references"]),
        "unsupported_reference_value_count": len(report["unsupported_reference_values"])}))
    if not report["complete_pages"] or report["unsupported_reference_values"]:
        raise SystemExit(2)


if __name__ == "__main__":
    main()
