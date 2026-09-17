#include "qsHypr.hpp"

#include <QDebug>
#include <QLocalSocket>
#include <QQmlEngine>
#include <QQmlExtensionPlugin>
#include <QStandardPaths>
#include <qqml.h>

// Resolve the command socket the same way hyprctl does:
// $XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock
static QString defaultSocketPath() {
	QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
	if (runtime.isEmpty())
		runtime = QStandardPaths::writableLocation(QStandardPaths::RuntimeLocation);

	const QString sig = qEnvironmentVariable("HYPRLAND_INSTANCE_SIGNATURE");
	if (runtime.isEmpty() || sig.isEmpty())
		return QString();

	return runtime + "/hypr/" + sig + "/.socket.sock";
}

QsHypr::QsHypr(QQmlEngine* engine, QObject* parent): QObject(parent), m_engine(engine) {
	m_socketPath = defaultSocketPath();
	if (m_socketPath.isEmpty())
		qWarning() << "QsHypr: HYPRLAND_INSTANCE_SIGNATURE/XDG_RUNTIME_DIR unset, IPC disabled";
}

void QsHypr::request(const QString& command, bool json, const QJSValue& callback) {
	sendRaw((json ? "j/" : "") + command, callback);
}

void QsHypr::dispatch(const QString& action, const QJSValue& callback) {
	sendRaw("dispatch " + action, callback);
}

// Hyprland evaluates command-socket connections synchronously and freezes
// until an unclosed connection times out (5s), so each request gets its own
// QLocalSocket that is connected, written, drained and destroyed in one pass.
// QLocalSocket has no half-close, so we accumulate the reply from readyRead
// and finalize on any completion signal (readChannelFinished / disconnected /
// errorOccurred — Hyprland replies without needing EOF, then closes).
void QsHypr::sendRaw(const QString& payload, const QJSValue& callback) {
	if (m_socketPath.isEmpty()) {
		if (callback.isCallable())
			callback.call({QJSValue(QString())});
		return;
	}

	QLocalSocket* sock = new QLocalSocket(this);
	QsHyprPendingRequest& pending = m_pending[sock];
	pending.callback = callback;

	connect(sock, &QLocalSocket::connected, this, [sock, payload]() {
		sock->write(payload.toUtf8());
		sock->flush();
	});

	connect(sock, &QLocalSocket::readyRead, this, [this, sock]() {
		QsHyprPendingRequest& p = m_pending[sock];
		p.buffer += sock->readAll();
	});

	auto finalize = [this, sock]() {
		auto it = m_pending.find(sock);
		if (it == m_pending.end())
			return; // already finalized
		QsHyprPendingRequest p = it.value();
		p.buffer += sock->readAll();
		m_pending.erase(it);
		sock->deleteLater();
		if (p.callback.isCallable())
			p.callback.call({QJSValue(QString::fromUtf8(p.buffer))});
	};

	connect(sock, &QLocalSocket::readChannelFinished, this, [finalize]() { finalize(); });
	connect(sock, &QLocalSocket::disconnected, this, [finalize]() { finalize(); });
	connect(sock, &QLocalSocket::errorOccurred, this, [this, sock, finalize](QLocalSocket::LocalSocketError err) {
		// PeerClosedError is expected: Hyprland closes after sending the reply.
		if (err != QLocalSocket::PeerClosedError)
			qWarning() << "QsHypr: socket error:" << sock->errorString();
		finalize();
	});

	sock->connectToServer(m_socketPath);
}

// Classic loadable QML plugin: registers QsHypr as the "QsHypr" singleton.
class QsHyprPlugin : public QQmlExtensionPlugin {
	Q_OBJECT;
	Q_PLUGIN_METADATA(IID QQmlExtensionInterface_iid);

public:
	void registerTypes(const char* uri) override {
		qmlRegisterSingletonType<QsHypr>(uri, 1, 0, "QsHypr", [](QQmlEngine* engine, QJSEngine*) {
			return new QsHypr(engine);
		});
	}
};

#include "qsHypr.moc"
