// Real acquisition/load qualification using the unchanged production collector and controller.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using System.Security.Cryptography;
using System.Text.Json;
using Whisper.NativeWatch;
using Whisper.NativeBuild;

public static class NativeBuildLiveLoad
{
    static Control NewControl(string root, string watched, string logs) => new Control(
        new[] { Path.Combine(root, "allowed-unused-1"), Path.Combine(root, "allowed-unused-2"), watched },
        new[] { Path.Combine(root, "protected-1"), Path.Combine(root, "protected-2"),
            Path.Combine(root, "protected-3"), Path.Combine(root, "protected-4") },
        Path.Combine(root, "user-temp"), logs);

    public static object ColdBurst(string root)
    {
        string watched = Path.Combine(root, "watched"), logs = Path.Combine(root, "logs");
        Directory.CreateDirectory(watched); Directory.CreateDirectory(logs);
        using var collector = new Collector(new[] { watched }, new[] { true });
        using var control = NewControl(root, watched, logs);
        var records = new Record[55414];
        for (int i = 0; i < records.Length; i++) records[i] = new Record { Root = watched,
            Name = "file-" + i, Session = "load", Volume = "v", Action = 3, FileId = (ulong)i + 1, At = "fixed" };
        // Inject only the finite load fixture into the existing bounded queue; never modify its limits.
        var publish = typeof(Collector).GetMethod("Publish", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic);
        publish.Invoke(collector, new object[] { records });
        var clock = Stopwatch.StartNew();
        var snapshot = collector.Drain();
        var failure = control.Process(snapshot);
        clock.Stop();
        return new { Status = failure == null && snapshot.Records.Length == 55414
                && snapshot.Events.Length == 55414 && clock.Elapsed.TotalMilliseconds + 100 <= 1000 ? "PASS" : "FAIL",
            Records = snapshot.Records.Length, Events = snapshot.Events.Length,
            ElapsedMs = clock.Elapsed.TotalMilliseconds, IntervalWithPauseMs = clock.Elapsed.TotalMilliseconds + 100,
            Scope = "First controller batch in a fresh process; synthetic queue injection, real Drain/Assemble/policy/journals/flush" };
    }

    public sealed class Result
    {
        public int ExpectedFiles, CreatedFiles, Drains;
        public long Records, Events;
        public double MaximumDrainIntervalMs, MaximumBatchMs;
        public bool SequenceUnique = true, CreatedUnique = true;
        public bool SequenceContiguous = true, JournalConserved;
        public long JournalRecords, JournalEvents;
        public string RecordDigest, EventDigest;
        public string Status = "STARTED", Failure;
        public List<double> DrainIntervalsMs = new List<double>();
        public List<double> BatchMs = new List<double>();
    }

    public static Result Run(string root, int count)
    {
        string watched = Path.Combine(root, "watched"), logs = Path.Combine(root, "logs");
        Directory.CreateDirectory(watched); Directory.CreateDirectory(logs);
        var result = new Result { ExpectedFiles = count };
        var created = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var sequences = new HashSet<long>();
        var json = new JsonSerializerOptions { IncludeFields = true };
        using var recordDigest = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
        using var eventDigest = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
        using var snapshotEventDigest = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
        long previousSequence = 0;
        var clock = Stopwatch.StartNew();
        double previousDrain = -1, lastEventAt = 0;
        Collector collector = null;
        Control control = null;
        Task writer = null;
        using var cancel = new CancellationTokenSource();

        void Drain()
        {
            double at = clock.Elapsed.TotalMilliseconds;
            if (previousDrain >= 0) {
                double interval = at - previousDrain;
                result.DrainIntervalsMs.Add(interval);
                result.MaximumDrainIntervalMs = Math.Max(result.MaximumDrainIntervalMs, interval);
            }
            previousDrain = at;
            var timer = Stopwatch.StartNew();
            Snapshot snapshot = collector.Drain();
            // Establish the expected digests before the controller can inspect the snapshot.
            foreach (var record in snapshot.Records) {
                result.Records++;
                if (!sequences.Add(record.Sequence)) result.SequenceUnique = false;
                if (record.Sequence != previousSequence + 1) result.SequenceContiguous = false;
                previousSequence = record.Sequence;
                recordDigest.AppendData(JsonSerializer.SerializeToUtf8Bytes(record, json));
            }
            foreach (var e in snapshot.Events) {
                result.Events++;
                if (e.Type == "Created" && !created.Add(e.Path)) result.CreatedUnique = false;
                eventDigest.AppendData(JsonSerializer.SerializeToUtf8Bytes(new { snapshot.Drain, e.Type, e.Path, e.OldPath, e.At, e.Records }, json));
                snapshotEventDigest.AppendData(JsonSerializer.SerializeToUtf8Bytes(e, json));
            }
            Observation failure = control.Process(snapshot);
            timer.Stop();
            result.Drains++;
            result.BatchMs.Add(timer.Elapsed.TotalMilliseconds);
            result.MaximumBatchMs = Math.Max(result.MaximumBatchMs, timer.Elapsed.TotalMilliseconds);
            if (snapshot.Records.Length != 0) lastEventAt = clock.Elapsed.TotalMilliseconds;
            if (failure != null) throw new InvalidDataException("Live control violation: " + failure.Detail);
        }

        try {
            collector = new Collector(new[] { watched }, new[] { true });
            control = NewControl(root, watched, logs);
            writer = Task.Run(() => {
                for (int i = 0; i < count; i++) {
                    cancel.Token.ThrowIfCancellationRequested();
                    File.WriteAllText(Path.Combine(watched, "file-" + i + ".txt"), "fixture " + i);
                }
            });
            do {
                Drain();
                if (clock.Elapsed.TotalSeconds > 90) throw new TimeoutException("Live load exceeded 90 seconds.");
                Thread.Sleep(100); // The production loop pause is unchanged.
            } while (!writer.IsCompleted || clock.Elapsed.TotalMilliseconds - lastEventAt < 500);
            writer.GetAwaiter().GetResult();
            collector.Dispose();
            Drain();
            control.Dispose(); // Finish the writer lifecycle before opening ordinary read-only readers.
            result.CreatedFiles = created.Count;
            for (int i = 0; i < count; i++)
                if (!created.Contains(Path.Combine(watched, "file-" + i + ".txt")))
                    throw new InvalidDataException("Missing Created identity: " + i);
            if (created.Count != count || !result.SequenceUnique || !result.CreatedUnique || !result.SequenceContiguous)
                throw new InvalidDataException("Live notification count/identity duplication or loss.");
            // Re-read both actual journals; preserve every acquired record and assembled observation,
            // including Changed, all fields, order, identities, and the drain number.
            result.RecordDigest = Convert.ToHexString(recordDigest.GetHashAndReset());
            result.EventDigest = Convert.ToHexString(eventDigest.GetHashAndReset());
            string expectedSnapshotEvents = Convert.ToHexString(snapshotEventDigest.GetHashAndReset());
            foreach (string line in File.ReadLines(Path.Combine(logs, "watcher-native-records.jsonl"))) {
                var snapshot = JsonSerializer.Deserialize<Snapshot>(line, json);
                foreach (var r in snapshot.Records) {
                    result.JournalRecords++;
                    recordDigest.AppendData(JsonSerializer.SerializeToUtf8Bytes(r, json));
                }
                foreach (var e in snapshot.Events)
                    snapshotEventDigest.AppendData(JsonSerializer.SerializeToUtf8Bytes(e, json));
            }
            foreach (string line in File.ReadLines(Path.Combine(logs, "watcher-events.jsonl"))) {
                using var document = JsonDocument.Parse(line);
                var row = document.RootElement;
                var raw = row.GetProperty("raw_event");
                var e = new Observation { Type = row.GetProperty("event").GetString(), Path = row.GetProperty("path").GetString(),
                    OldPath = row.GetProperty("old_path").GetString(), At = row.GetProperty("event_at").GetString(),
                    Records = raw.GetProperty("records").Deserialize<Record[]>(json) };
                var n = e.Records[e.Type == "Renamed" ? 1 : 0];
                if (row.GetProperty("mode").GetString() != "NativeBuild" || row.GetProperty("disposition").GetString() != "ALLOWED"
                    || raw.GetProperty("watcher_root").GetString() != n.Root
                    || raw.GetProperty("args_type").GetString() != "Win32.FILE_NOTIFY_EXTENDED_INFORMATION"
                    || raw.GetProperty("raw_name").GetString() != n.Name
                    || raw.GetProperty("name_is_null").GetBoolean() != (n.Name == null)
                    || raw.GetProperty("name_is_empty").GetBoolean() != (n.Name == "")
                    || raw.GetProperty("raw_old_name").GetString() != (e.Type == "Renamed" ? e.Records[0].Name : null)
                    || raw.GetProperty("file_id").GetString() != n.FileId.ToString("x16")
                    || raw.GetProperty("volume").GetString() != n.Volume || raw.GetProperty("session").GetString() != n.Session)
                    throw new InvalidDataException("Live journal envelope changed.");
                result.JournalEvents++;
                long drainNumber = raw.GetProperty("drain").GetInt64();
                eventDigest.AppendData(JsonSerializer.SerializeToUtf8Bytes(new { Drain = drainNumber, e.Type, e.Path, e.OldPath, e.At, e.Records }, json));
            }
            result.JournalConserved = result.JournalRecords == result.Records && result.JournalEvents == result.Events
                && Convert.ToHexString(recordDigest.GetHashAndReset()) == result.RecordDigest
                && Convert.ToHexString(eventDigest.GetHashAndReset()) == result.EventDigest
                && Convert.ToHexString(snapshotEventDigest.GetHashAndReset()) == expectedSnapshotEvents;
            if (!result.JournalConserved) throw new InvalidDataException("Live journals lost, duplicated or changed acquired records/events.");
            if (result.MaximumBatchMs > 1000 || result.MaximumDrainIntervalMs > 1000)
                throw new InvalidDataException("Live acquisition/control/journal cadence exceeds 1000 ms.");
            result.Status = "PASS";
        }
        catch (Exception e) { result.Status = "FAIL"; result.Failure = e.ToString(); }
        finally {
            cancel.Cancel();
            // All cleanup attempts are independent, and their errors keep the gate failed.
            void Cleanup(Action action) {
                try { action(); } catch (Exception e) { result.Status = "FAIL"; result.Failure += "\nCleanup: " + e; }
            }
            if (writer != null) Cleanup(() => {
                try { if (!writer.Wait(TimeSpan.FromSeconds(10))) throw new TimeoutException("Fixture writer did not close."); }
                catch (AggregateException e) when (e.InnerException is OperationCanceledException) { }
            });
            if (collector != null) Cleanup(() => collector.Dispose());
            if (control != null) Cleanup(() => control.Dispose());
            result.CreatedFiles = created.Count;
        }
        return result;
    }
}
