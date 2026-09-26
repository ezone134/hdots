#pragma once
#include <QJSEngine>
#include <QObject>
#include <QQmlEngine>
#include <QString>

///! In-process file I/O for Quickshell (no external programs).
class QsIo : public QObject {
	Q_OBJECT;

public:
	explicit QsIo(QObject* parent = nullptr);

	/// Overwrite `path` with `content` (atomic via QSaveFile).
	Q_INVOKABLE void writeFile(const QString& path, const QString& content);
	/// Append `content` to `path`.
	Q_INVOKABLE void appendFile(const QString& path, const QString& content);
	/// Read the whole file, or empty string on failure.
	Q_INVOKABLE QString readFile(const QString& path);
};
