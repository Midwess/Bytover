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
            if isPasswordEditable() || !(cloud?.password?.isEmpty ?? true) {
                editPassword
            }

            if cloud != nil {
                accessUrl
            }

            Spacer()
                .frame(height: SpaceTheme.item.value)

            Button(action: {
                // Upload action
                Task {
                    if let cloud = cloud {
                        await core.update(.transfer(.cancelTransfer(session_id: cloud.session_id)))
                    } else {
                        await core.update(.transfer(.startPublicTransfer(password: password.isEmpty ? nil : password)))
                    }
                }
            }) {
                if let cloud = cloud {
                    if cloud.is_in_progress {
                        HStack(spacing: SpaceTheme.item.value) {
                            HStack(spacing: 1) {
                                Text(cloud.display_download_speed)
                                    .modifier(Label1())
                                    .foregroundColor(Theme.PrimaryText.color)
                                ImageAsset.Upload.image
                                    .frame(width: 10, height: 10)
                            }
                            .frame(width: 100)
                            SharingProgress(progress: cloud.progress)
                                .foregroundColor(Theme.PrimaryText.color)
                                .frame(width: 25, height: 25)
                        }
                    } else {
                        Text("Continue")
                            .modifier(Label1())
                            .foregroundColor(Theme.PrimaryText.color)
                    }
                } else {
                    Text("Upload")
                        .modifier(Label1())
                        .foregroundColor(Theme.PrimaryText.color)
                }
            }
            .padding(.vertical, SpaceTheme.item.value)
            .padding(.horizontal, SpaceTheme.screen.value)
            .background((cloud?.is_completed ?? false) ? Theme.GreenSecondary.color.opacity(0.6) : (cloud == nil ? Theme.BluePrimary.color : Theme.PrimaryText.color.opacity(0.1)))
            .clipShape(Capsule())

            Spacer()
        }
        .onReceive(core.cloudSession, perform: { value in
            cloud = value
            password = cloud?.password ?? ""
        })
    }

    func isPasswordEditable() -> Bool {
        return self.cloud == nil
    }

    var accessUrl: some View {
        VStack(alignment: .leading) {
            HStack(spacing: SpaceTheme.cohesive.value) {
                ImageAsset.Link.image
                    .foregroundColor(Theme.PrimaryText.color.opacity(0.8))
                VStack {
                    Text(verbatim: cloud?.access_url ?? "Generating..")
                        .underline()
                        .foregroundColor(Theme.PrimaryText.color.opacity(1))
                        .truncationMode(.middle)
                        .frame(height: SpaceTheme.screen.value)
                        .modifier(Label1())
                        .foregroundColor(Theme.PrimaryText.color)
                        .autocapitalization(.none)
                }
                Spacer()
            }
        }
        .padding(.vertical, SpaceTheme.item.value)
        .padding(.horizontal, SpaceTheme.screen.value)
        .frame(maxWidth: .infinity, idealHeight: 30)
        .background(Theme.PrimaryText.color.opacity(0.16))
        .clipShape(Capsule())
    }

    var editPassword: some View {
        HStack(spacing: SpaceTheme.cohesive.value) {
            ImageAsset.Lock.image
                .foregroundColor(Theme.PrimaryText.color.opacity(0.8))
            Spacer()
            ZStack {
                SecureField("Enter password (optional)", text: $password)
                    .disabled(!isPasswordEditable())
                    .frame(height: SpaceTheme.screen.value)
                    .modifier(Label1())
                    .foregroundColor(Theme.PrimaryText.color)
                    .autocapitalization(.none)
                    .disableAutocorrection(true)
                    .focused($isTextFieldFocused)
                    .opacity(isObfuscated ? (isPasswordEditable() ? 1 : 0.8) : 0)
                    .onChange(of: password) { newValue in
                        if newValue.count > 20 {
                            password = String(newValue.prefix(20))
                        }
                    }

                TextField("Enter password (optional)", text: $password)
                    .disabled(!isPasswordEditable())
                    .frame(height: SpaceTheme.screen.value)
                    .modifier(Label1())
                    .foregroundColor(Theme.PrimaryText.color)
                    .autocapitalization(.none)
                    .disableAutocorrection(true)
                    .focused($isTextFieldFocused)
                    .opacity(isObfuscated ? 0 : (isPasswordEditable() ? 1 : 0.8))
                    .onChange(of: password) { newValue in
                        if newValue.count > 20 {
                            password = String(newValue.prefix(20))
                        }
                    }
            }

            if !password.isEmpty && isPasswordEditable() {
                Button(action: {
                    password = ""
                    isTextFieldFocused = true
                }) {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundColor(Theme.PrimaryText.color.opacity(0.6))
                }
            }

            if !password.isEmpty {
                Button(action: {
                    isObfuscated.toggle()
                    isTextFieldFocused = true
                }) {
                    Image(systemName: isObfuscated ? "eye.slash" : "eye")
                        .foregroundColor(Theme.PrimaryText.color.opacity(0.6))
                }
            }
        }
        .padding(.vertical, SpaceTheme.item.value)
        .padding(.horizontal, SpaceTheme.screen.value)
        .frame(maxWidth: .infinity, idealHeight: 30)
        .background(Theme.PrimaryText.color.opacity(0.16))
        .clipShape(Capsule())
    }
}
