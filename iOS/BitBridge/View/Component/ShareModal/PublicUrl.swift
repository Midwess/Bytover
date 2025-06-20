import SwiftUI
import Foundation
import SharedTypes

struct PublicUrlShareView: View {
    @EnvironmentObject var core: Core
    @State private var password: String = ""
    @State private var isObfuscated: Bool = true
    @State private var cloud: CloudSession?
    @FocusState private var isTextFieldFocused: Bool

    var body: some View {
        VStack(spacing: SpaceTheme.item.value) {
            Text("Password-protected shareable URL.")
                .modifier(Label1())
                .foregroundColor(Theme.PrimaryText.color.opacity(0.7))
                .multilineTextAlignment(.leading)

            HStack {
                ZStack {
                    SecureField("Enter password (optional)", text: $password)
                        .frame(height: 22)
                        .modifier(Label1())
                        .foregroundColor(Theme.PrimaryText.color)
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                        .focused($isTextFieldFocused)
                        .opacity(isObfuscated ? 1 : 0)
                        .onChange(of: password) { newValue in
                            if newValue.count > 20 {
                                password = String(newValue.prefix(20))
                            }
                        }

                    TextField("Enter password (optional)", text: $password)
                        .frame(height: 22)
                        .modifier(Label1())
                        .foregroundColor(Theme.PrimaryText.color)
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                        .focused($isTextFieldFocused)
                        .opacity(isObfuscated ? 0 : 1)
                        .onChange(of: password) { newValue in
                            if newValue.count > 20 {
                                password = String(newValue.prefix(20))
                            }
                        }
                }
                .frame(width: .infinity)

                if !password.isEmpty {
                    Button(action: {
                        password = ""
                        isTextFieldFocused = true
                    }) {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundColor(Theme.PrimaryText.color.opacity(0.6))
                    }
                }

                Button(action: {
                    isObfuscated.toggle()
                    isTextFieldFocused = true
                }) {
                    Image(systemName: isObfuscated ? "eye.slash" : "eye")
                        .foregroundColor(Theme.PrimaryText.color.opacity(0.6))
                }
            }
            .padding(.vertical, 12)
            .padding(.horizontal, 20)
            .frame(maxWidth: .infinity)
            .background(Theme.PrimaryText.color.opacity(0.15))
            .clipShape(Capsule())

            Spacer()
                .frame(height: 10)

            Button(action: {
                // Upload action
                Task {
                    await core.update(.transfer(.startPublicTransfer(password: password.isEmpty ? nil : password)))
                }
            }) {
                Text("Upload \(cloud?.display_download_speed)")
                    .modifier(Label1())
                    .foregroundColor(Theme.PrimaryText.color)
            }
            .padding(.vertical, 12)
            .padding(.horizontal, 20)
            .background(Theme.BluePrimary.color)
            .clipShape(Capsule())
        }
        .onReceive(core.cloudSession, perform: { value in
            cloud = value
            password = cloud?.password ?? ""
        })
    }
}
