import copy
import unittest

from snapshot_audit import audit


def page(kind, items):
    return {kind: items, "has_more": False, "total": len(items)}


def snapshot():
    field = {"field_id": "relation", "field_title": "来源", "field_type": "twoWayLinkRecords",
             "property_two_way_link_records": {"sub_id": "sessions"}}
    return {"sheets": {
        "goals": {"fields": page("fields", [field]), "views": page("views", []),
                  "records": page("records", [{"record_id": "goal", "field_values": [
                      {"field": "来源", "reference_value": {"items": ["session"]}}]}])},
        "sessions": {"fields": page("fields", []), "views": page("views", []),
                     "records": page("records", [{"record_id": "session", "field_values": []}])},
    }}


class SnapshotAuditTests(unittest.TestCase):
    def test_field_titles_and_ids_produce_same_reference(self):
        original = snapshot()
        by_id = copy.deepcopy(original)
        by_id["sheets"]["goals"]["records"]["records"][0]["field_values"][0]["field"] = "relation"
        self.assertEqual(audit(original), audit(by_id))
        self.assertEqual(audit(original)["reference_edge_count"], 1)
        self.assertFalse(audit(original)["unresolved_references"])

    def test_missing_pagination_and_duplicates_cannot_establish_complete_export(self):
        source = snapshot()
        del source["sheets"]["goals"]["records"]["has_more"]
        self.assertFalse(audit(source)["complete_pages"])
        source = snapshot()
        source["sheets"]["goals"]["records"]["records"].append(copy.deepcopy(source["sheets"]["goals"]["records"]["records"][0]))
        source["sheets"]["goals"]["records"]["total"] = 2
        self.assertFalse(audit(source)["complete_pages"])

    def test_removed_table_is_separate_from_missing_record(self):
        source = snapshot()
        source["sheets"]["sessions"]["records"] = page("records", [])
        report = audit(source)
        self.assertEqual(report["unresolved_references"][0]["reason"], "missing_record")
        del source["sheets"]["sessions"]
        report = audit(source)
        self.assertEqual(report["unresolved_references"][0]["reason"], "missing_sheet")
        self.assertEqual(len(report["obsolete_relation_fields"]), 1)

    def test_ambiguous_titles_and_unsupported_ids_are_not_guessed(self):
        source = snapshot()
        field = copy.deepcopy(source["sheets"]["goals"]["fields"]["fields"][0])
        field["field_id"] = "other"
        source["sheets"]["goals"]["fields"] = page("fields", [field, source["sheets"]["goals"]["fields"]["fields"][0]])
        report = audit(source)
        self.assertFalse(report["complete_pages"])
        self.assertEqual(report["reference_edge_count"], 0)
        self.assertTrue(report["unsupported_reference_values"])
        source = snapshot()
        source["sheets"]["goals"]["records"]["records"][0]["field_values"][0]["reference_value"]["items"] = [42, {"id": 3}]
        self.assertEqual(len(audit(source)["unsupported_reference_values"]), 2)
        self.assertEqual(audit(source)["reference_edge_count"], 0)


if __name__ == "__main__":
    unittest.main()
