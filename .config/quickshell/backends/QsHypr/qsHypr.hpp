#pragma once

#include <QByteArray>
#include <QHash>
#include <QJSValue>
#include <QObject>
#include <QString>

class QLocalSocket;
class QQmlEngine;

struct QsHyprPendingRequest {
	QJSValue callback;
	QByteArray buffer;
};

// In-process Hyprland IPC client (Hyprland 0.55 socket protocol).
// Replaces every `hyprctl` fork: queries are async via a per-request
// QLocalSocket (open, write, read, close — the docs mandate closing the
// connection immediately, connections are evaluated synchronously). Zero
// forks, zero threads, fully event-driven. Import as `import QsHypr 1.0`.
class QsHypr : public QObject {
	Q_OBJECT
public:
	explicit QsHypr(QQmlEngine* engine, QObject* parent = nullptr);

	// Query. json=true prefixes the payload with "j/" so Hyprland replies
	// with JSON. callback receives the raw response string ("" on error).
	Q_INVOKABLE void request(const QString& command, bool json = false, const QJSValue& callback = QJSValue());

	// Fire-and-forget action, e.g. dispatch("workspace 2"). callback (if
	// given) fires with the "ok"/error text once Hyprland responds.
	Q_INVOKABLE void dispatch(const QString& action, const QJSValue& callback = QJSValue());

	Q_PROPERTY(QString socketPath READ socketPath CONSTANT)
	QString socketPath() const { return m_socketPath; }

private:
	void sendRaw(const QString& payload, const QJSValue& callback);

	QString m_socketPath;
	QQmlEngine* m_engine = nullptr;
	QHash<QLocalSocket*, QsHyprPendingRequest> m_pending;
};
