// NativeBuild policy/journaling hot path. Notification acquisition and pairing stay in Collector.
using System;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using Whisper.NativeWatch;

namespace Whisper.NativeBuild
{
    public sealed class Control : IDisposable
    {
        static readonly JsonSerializerOptions Json = new JsonSerializerOptions { IncludeFields = true };
        readonly string[] allowed, protectedRoots;
        readonly string temp;
        readonly TextWriter records, events, unattributed, failures;
        bool disposed;

        public Control(string[] allowed, string[] protectedRoots, string temp, string logRoot)
        {
            this.allowed = allowed.Select(Full).ToArray();
            this.protectedRoots = protectedRoots.Select(Full).ToArray();
            this.temp = Full(temp);
            // Close every writer already opened if a subsequent open fails.
            try {
                records = Open(logRoot, "watcher-native-records.jsonl");
                events = Open(logRoot, "watcher-events.jsonl");
                unattributed = Open(logRoot, "unattributed-temp.jsonl");
                failures = Open(logRoot, "guard-events.jsonl");
            } catch { Dispose(); throw; }
        }

        // Injection of journal failures is test-only and does not alter notification policy.
        public Control(string[] allowed, string[] protectedRoots, string temp,
            TextWriter records, TextWriter events, TextWriter unattributed, TextWriter failures)
        {
            this.allowed = allowed.Select(Full).ToArray();
            this.protectedRoots = protectedRoots.Select(Full).ToArray();
            this.temp = Full(temp);
            this.records = records; this.events = events;
            this.unattributed = unattributed; this.failures = failures;
        }

        static StreamWriter Open(string root, string name) => new StreamWriter(
            new FileStream(Path.Combine(root, name), FileMode.Append, FileAccess.Write, FileShare.Read),
            new UTF8Encoding(false)) { NewLine = "\n" };

        static string Full(string path)
        {
            if (string.IsNullOrWhiteSpace(path)) throw new ArgumentException("A path is required.");
            return Path.GetFullPath(path).TrimEnd('\\');
        }
        static bool In(string path, string root) => path.Equals(root, StringComparison.OrdinalIgnoreCase)
            || path.StartsWith(root + "\\", StringComparison.OrdinalIgnoreCase);

        // Exact precedence of Get-EventDisposition, including Temp before protected roots.
        public string Disposition(Observation e)
        {
            if (string.Equals(e.Type, "WatcherError", StringComparison.OrdinalIgnoreCase)) return "VIOLATION";
            string path = Full(e.Path);
            bool rename = string.Equals(e.Type, "Renamed", StringComparison.OrdinalIgnoreCase);
            string old = rename && !string.IsNullOrWhiteSpace(e.OldPath) ? Full(e.OldPath) : "";
            bool newTemp = In(path, temp), oldTemp = old.Length != 0 && In(old, temp);
            if (rename) {
                if (newTemp && oldTemp) return "UNATTRIBUTED_TEMP";
                if (newTemp || oldTemp) return "VIOLATION";
            } else if (newTemp && new[] { "Created", "Changed", "Deleted" }.Contains(e.Type, StringComparer.OrdinalIgnoreCase))
                return "UNATTRIBUTED_TEMP";
            if (protectedRoots.Any(r => In(path, r)) || !allowed.Any(r => In(path, r))) return "VIOLATION";
            if (rename) {
                // The reference rejects an empty old path by exception when it reaches root checks.
                old = Full(old);
                if (!allowed.Any(r => In(old, r)) || protectedRoots.Any(r => In(old, r))) return "VIOLATION";
            }
            return "ALLOWED";
        }

        static void Line(TextWriter writer, object value) => writer.WriteLine(JsonSerializer.Serialize(value, Json));

        // One finite snapshot, in Collector order. Stop at the first violation, as before.
        public Observation Process(Snapshot snapshot)
        {
            if (disposed) throw new ObjectDisposedException(nameof(Control));
            if (snapshot.Records.Length != 0) Line(records, snapshot);
            foreach (Observation e in snapshot.Events) {
                bool rename = e.Type == "Renamed";
                Record n = e.Records[rename ? 1 : 0], o = rename ? e.Records[0] : null;
                var raw = new {
                    watcher_root = n.Root, args_type = "Win32.FILE_NOTIFY_EXTENDED_INFORMATION",
                    raw_name = n.Name, name_is_null = n.Name == null, name_is_empty = n.Name == "",
                    raw_old_name = o?.Name, file_id = n.FileId.ToString("x16"), volume = n.Volume,
                    session = n.Session, drain = snapshot.Drain, records = e.Records
                };
                string disposition = Disposition(e);
                Line(events, new { mode = "NativeBuild", path = e.Path, old_path = e.OldPath,
                    @event = e.Type, event_at = e.At, disposition, raw_event = raw });
                if (disposition == "UNATTRIBUTED_TEMP")
                    Line(unattributed, new { mode = "NativeBuild", path = e.Path, old_path = e.OldPath,
                        @event = e.Type, event_at = e.At, detected_at = DateTimeOffset.UtcNow.ToString("o"),
                        attribution = "UNATTRIBUTED", detail = e.Detail, raw_event = raw });
                if (disposition == "VIOLATION") {
                    Line(failures, new { mode = "NativeBuild", status = "FAIL", path = e.Path, old_path = e.OldPath,
                        @event = e.Type, event_at = e.At, detected_at = DateTimeOffset.UtcNow.ToString("o"),
                        action = "campaign-process-tree-terminated", detail = e.Detail, raw_event = raw });
                    Flush();
                    return e;
                }
            }
            Flush();
            return null;
        }

        public void Flush()
        {
            // Attempt every flush even if one fails; all failures propagate.
            var errors = new System.Collections.Generic.List<Exception>();
            foreach (TextWriter writer in new[] { records, events, unattributed, failures })
                try { writer?.Flush(); } catch (Exception e) { errors.Add(e); }
            if (errors.Count != 0) throw new AggregateException("NativeBuild journal flush failed.", errors);
        }
        public void Dispose()
        {
            if (disposed) return;
            disposed = true;
            var errors = new System.Collections.Generic.List<Exception>();
            foreach (TextWriter writer in new[] { records, events, unattributed, failures })
                try { writer?.Dispose(); } catch (Exception e) { errors.Add(e); }
            if (errors.Count != 0) throw new AggregateException("NativeBuild journal close failed.", errors);
        }
    }
}
